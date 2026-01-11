//! Comprehensive tests for device handling scenarios
//!
//! This test module covers:
//!
//! **Device Hot-Plug:**
//! 1. Device disconnection during playback
//! 2. Device reconnection and recovery
//! 3. Default device changes
//! 4. Multiple device enumeration
//! 5. Switching between devices mid-playback
//!
//! **Sample Rate Switching:**
//! 1. Device reports different sample rate than requested
//! 2. Dynamic sample rate changes from device
//! 3. Automatic resampling when device rate differs
//! 4. Sample rate negotiation
//!
//! ## Test Strategy
//!
//! Hardware-dependent tests are marked with `#[ignore]` by default and can be run with:
//! ```bash
//! cargo test -p soul-audio-desktop device_handling_test -- --ignored
//! ```
//!
//! Mock-based tests run without hardware and validate logic.
//!
//! Run all tests: `cargo test -p soul-audio-desktop device_handling_test`

use soul_audio_desktop::{
    backend::{list_available_backends, AudioBackend},
    device::{
        find_device_by_name, get_default_device, get_default_device_with_capabilities,
        list_devices, list_devices_with_capabilities, DeviceCapabilities, DeviceError,
        SupportedBitDepth, STANDARD_SAMPLE_RATES,
    },
    playback::{DesktopPlayback, PlaybackCommand},
    sources::local::LocalAudioSource,
};
use soul_playback::{AudioSource, PlaybackConfig, QueueTrack, TrackSource};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// ============================================================================
// Mock Device Types for Unit Testing
// ============================================================================

/// Mock device state for testing device handling logic without hardware
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockDeviceState {
    Connected,
    Disconnected,
    Reconnecting,
    Error(String),
}

/// Mock device for testing device handling scenarios
#[derive(Debug, Clone)]
pub struct MockDevice {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub state: MockDeviceState,
    pub capabilities: DeviceCapabilities,
}

impl MockDevice {
    /// Create a new mock device with default settings
    pub fn new(name: &str, sample_rate: u32) -> Self {
        Self {
            name: name.to_string(),
            sample_rate,
            channels: 2,
            state: MockDeviceState::Connected,
            capabilities: DeviceCapabilities {
                sample_rates: vec![44100, 48000, 96000, 192000],
                bit_depths: vec![SupportedBitDepth::Int16, SupportedBitDepth::Float32],
                max_channels: 2,
                supports_exclusive: false,
                supports_dsd: false,
                dsd_rates: vec![],
                min_buffer_frames: Some(256),
                max_buffer_frames: Some(4096),
                has_hardware_volume: false,
            },
        }
    }

    /// Create a high-end audio interface mock
    pub fn high_end_dac(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sample_rate: 96000,
            channels: 2,
            state: MockDeviceState::Connected,
            capabilities: DeviceCapabilities {
                sample_rates: vec![44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000],
                bit_depths: vec![
                    SupportedBitDepth::Int16,
                    SupportedBitDepth::Int24,
                    SupportedBitDepth::Int32,
                    SupportedBitDepth::Float32,
                ],
                max_channels: 2,
                supports_exclusive: true,
                supports_dsd: true,
                dsd_rates: vec![2822400, 5644800],
                min_buffer_frames: Some(64),
                max_buffer_frames: Some(8192),
                has_hardware_volume: true,
            },
        }
    }

    /// Simulate device disconnection
    pub fn disconnect(&mut self) {
        self.state = MockDeviceState::Disconnected;
    }

    /// Simulate device reconnection
    pub fn reconnect(&mut self) {
        self.state = MockDeviceState::Connected;
    }

    /// Change sample rate (simulates driver config change)
    pub fn set_sample_rate(&mut self, rate: u32) {
        if self.capabilities.sample_rates.contains(&rate) {
            self.sample_rate = rate;
        }
    }

    /// Check if device is available
    pub fn is_available(&self) -> bool {
        self.state == MockDeviceState::Connected
    }
}

/// Mock device manager for testing device enumeration and switching
pub struct MockDeviceManager {
    devices: Vec<MockDevice>,
    default_device_index: Option<usize>,
}

impl MockDeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            default_device_index: None,
        }
    }

    pub fn add_device(&mut self, device: MockDevice) {
        if self.devices.is_empty() {
            self.default_device_index = Some(0);
        }
        self.devices.push(device);
    }

    pub fn remove_device(&mut self, name: &str) -> Option<MockDevice> {
        if let Some(pos) = self.devices.iter().position(|d| d.name == name) {
            let device = self.devices.remove(pos);
            // Update default index
            if let Some(idx) = self.default_device_index {
                if pos == idx {
                    self.default_device_index = if self.devices.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                } else if pos < idx {
                    self.default_device_index = Some(idx - 1);
                }
            }
            Some(device)
        } else {
            None
        }
    }

    pub fn get_device(&self, name: &str) -> Option<&MockDevice> {
        self.devices.iter().find(|d| d.name == name)
    }

    pub fn get_device_mut(&mut self, name: &str) -> Option<&mut MockDevice> {
        self.devices.iter_mut().find(|d| d.name == name)
    }

    pub fn get_default_device(&self) -> Option<&MockDevice> {
        self.default_device_index
            .and_then(|idx| self.devices.get(idx))
    }

    pub fn set_default_device(&mut self, name: &str) -> bool {
        if let Some(pos) = self.devices.iter().position(|d| d.name == name) {
            self.default_device_index = Some(pos);
            true
        } else {
            false
        }
    }

    pub fn list_devices(&self) -> Vec<&MockDevice> {
        self.devices.iter().filter(|d| d.is_available()).collect()
    }

    pub fn list_all_devices(&self) -> &[MockDevice] {
        &self.devices
    }
}

impl Default for MockDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test WAV file
fn create_test_wav(path: &Path, duration_secs: f32, frequency: f32, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    use hound::{WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        let amplitude = (i16::MAX as f32 * 0.5 * sample) as i16;
        writer.write_sample(amplitude)?;
        writer.write_sample(amplitude)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Check if audio hardware is available
fn has_audio_hardware() -> bool {
    match list_devices(AudioBackend::Default) {
        Ok(devices) => !devices.is_empty(),
        Err(_) => false,
    }
}

/// Get the first available audio device name
fn get_first_device_name() -> Option<String> {
    list_devices(AudioBackend::Default)
        .ok()
        .and_then(|d| d.first().map(|dev| dev.name.clone()))
}

// ============================================================================
// 1. Mock Device Unit Tests
// ============================================================================

mod mock_device_tests {
    use super::*;

    #[test]
    fn test_mock_device_creation() {
        let device = MockDevice::new("Test Device", 44100);
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.sample_rate, 44100);
        assert_eq!(device.channels, 2);
        assert!(device.is_available());
    }

    #[test]
    fn test_mock_device_disconnect() {
        let mut device = MockDevice::new("Test Device", 48000);
        assert!(device.is_available());

        device.disconnect();
        assert!(!device.is_available());
        assert_eq!(device.state, MockDeviceState::Disconnected);
    }

    #[test]
    fn test_mock_device_reconnect() {
        let mut device = MockDevice::new("Test Device", 48000);
        device.disconnect();
        assert!(!device.is_available());

        device.reconnect();
        assert!(device.is_available());
        assert_eq!(device.state, MockDeviceState::Connected);
    }

    #[test]
    fn test_mock_device_sample_rate_change() {
        let mut device = MockDevice::new("Test Device", 44100);
        assert_eq!(device.sample_rate, 44100);

        device.set_sample_rate(96000);
        assert_eq!(device.sample_rate, 96000);

        // Invalid rate should not change
        device.set_sample_rate(12345); // Not in capabilities
        assert_eq!(device.sample_rate, 96000);
    }

    #[test]
    fn test_mock_high_end_dac() {
        let device = MockDevice::high_end_dac("Audiophile DAC");
        assert_eq!(device.sample_rate, 96000);
        assert!(device.capabilities.supports_exclusive);
        assert!(device.capabilities.supports_dsd);
        assert!(!device.capabilities.dsd_rates.is_empty());
    }

    #[test]
    fn test_mock_device_manager_add_remove() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Device 1", 44100));
        manager.add_device(MockDevice::new("Device 2", 48000));

        assert_eq!(manager.list_all_devices().len(), 2);

        let removed = manager.remove_device("Device 1");
        assert!(removed.is_some());
        assert_eq!(manager.list_all_devices().len(), 1);
    }

    #[test]
    fn test_mock_device_manager_default_device() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Device 1", 44100));
        manager.add_device(MockDevice::new("Device 2", 48000));

        // First device should be default
        let default = manager.get_default_device();
        assert!(default.is_some());
        assert_eq!(default.unwrap().name, "Device 1");

        // Change default
        manager.set_default_device("Device 2");
        let default = manager.get_default_device();
        assert_eq!(default.unwrap().name, "Device 2");
    }

    #[test]
    fn test_mock_device_manager_list_only_available() {
        let mut manager = MockDeviceManager::new();

        let mut dev1 = MockDevice::new("Device 1", 44100);
        let dev2 = MockDevice::new("Device 2", 48000);

        dev1.disconnect(); // Device 1 is disconnected

        manager.add_device(dev1);
        manager.add_device(dev2);

        // list_devices should only return connected devices
        let available = manager.list_devices();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "Device 2");
    }

    #[test]
    fn test_mock_device_manager_remove_default() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Device 1", 44100));
        manager.add_device(MockDevice::new("Device 2", 48000));

        // Remove default device
        manager.remove_device("Device 1");

        // New default should be the remaining device
        let default = manager.get_default_device();
        assert!(default.is_some());
        assert_eq!(default.unwrap().name, "Device 2");
    }
}

// ============================================================================
// 2. Device Disconnection During Playback (Integration)
// ============================================================================

mod device_disconnection_tests {
    use super::*;

    /// Test device disconnection scenario (mock-based logic test)
    #[test]
    fn test_mock_device_disconnection_logic() {
        let mut manager = MockDeviceManager::new();
        manager.add_device(MockDevice::new("Primary Device", 48000));
        manager.add_device(MockDevice::new("Fallback Device", 44100));

        // Simulate playback starting on primary
        let primary = manager.get_device("Primary Device").unwrap();
        assert!(primary.is_available());

        // Simulate disconnection
        manager.get_device_mut("Primary Device").unwrap().disconnect();

        // Primary should no longer be available
        let primary = manager.get_device("Primary Device").unwrap();
        assert!(!primary.is_available());

        // Fallback should still be available
        let fallback = manager.get_device("Fallback Device").unwrap();
        assert!(fallback.is_available());
    }

    /// Test graceful handling when device list changes
    #[test]
    fn test_mock_device_list_changes() {
        let mut manager = MockDeviceManager::new();

        // Start with 3 devices
        manager.add_device(MockDevice::new("Device A", 44100));
        manager.add_device(MockDevice::new("Device B", 48000));
        manager.add_device(MockDevice::new("Device C", 96000));

        assert_eq!(manager.list_devices().len(), 3);

        // Remove one
        manager.remove_device("Device B");
        assert_eq!(manager.list_devices().len(), 2);

        // Disconnect another
        manager.get_device_mut("Device A").unwrap().disconnect();
        assert_eq!(manager.list_devices().len(), 1);

        // Last one standing
        let remaining: Vec<_> = manager.list_devices();
        assert_eq!(remaining[0].name, "Device C");
    }

    /// Test device disconnection during actual playback
    /// Requires actual audio hardware
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_real_device_disconnection_during_playback() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(playback) => {
                let device_name = playback.get_current_device();
                eprintln!("Current device: {}", device_name);

                // In a real test, we would:
                // 1. Start playback
                // 2. Disconnect the device (physically or via OS)
                // 3. Verify error handling
                // 4. Verify recovery to fallback device

                eprintln!("To test device disconnection:");
                eprintln!("  1. Start playback");
                eprintln!("  2. Unplug or disable the audio device");
                eprintln!("  3. Observe error handling behavior");
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }
}

// ============================================================================
// 3. Device Reconnection and Recovery Tests
// ============================================================================

mod device_reconnection_tests {
    use super::*;

    #[test]
    fn test_mock_device_reconnection_flow() {
        let mut device = MockDevice::new("USB DAC", 96000);

        // Initial state
        assert!(device.is_available());

        // Disconnect
        device.disconnect();
        assert!(!device.is_available());

        // Reconnect
        device.reconnect();
        assert!(device.is_available());

        // Verify capabilities remain after reconnection
        assert!(device.capabilities.sample_rates.contains(&96000));
    }

    #[test]
    fn test_mock_reconnection_preserves_settings() {
        let mut device = MockDevice::high_end_dac("Pro Audio Interface");

        // Set to specific sample rate
        device.set_sample_rate(192000);

        // Disconnect and reconnect
        device.disconnect();
        device.reconnect();

        // Sample rate should be preserved
        assert_eq!(device.sample_rate, 192000);
    }

    #[test]
    fn test_mock_manager_handles_device_cycling() {
        let mut manager = MockDeviceManager::new();
        manager.add_device(MockDevice::new("Audio Device", 48000));

        // Cycle device state multiple times
        for i in 0..5 {
            let device = manager.get_device_mut("Audio Device").unwrap();

            if i % 2 == 0 {
                device.disconnect();
                assert!(!device.is_available());
            } else {
                device.reconnect();
                assert!(device.is_available());
            }
        }

        // Final state should be disconnected (odd iterations)
        let device = manager.get_device("Audio Device").unwrap();
        assert!(!device.is_available());
    }

    /// Test real device reconnection behavior
    #[test]
    #[ignore = "Requires audio hardware and manual device cycling"]
    fn test_real_device_reconnection() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        eprintln!("=== Device Reconnection Test ===");
        eprintln!("This test requires manual intervention:");
        eprintln!("  1. Note the current device list");
        eprintln!("  2. Disconnect a USB audio device");
        eprintln!("  3. Wait 2 seconds");
        eprintln!("  4. Reconnect the device");
        eprintln!("  5. Verify it appears in the device list again");

        let devices_before = list_devices(AudioBackend::Default).unwrap_or_default();
        eprintln!("Devices before: {:?}", devices_before.iter().map(|d| &d.name).collect::<Vec<_>>());

        // In automated testing, we would use system APIs to simulate device events
        // For now, this serves as a manual test guide
    }
}

// ============================================================================
// 4. Default Device Change Tests
// ============================================================================

mod default_device_change_tests {
    use super::*;

    #[test]
    fn test_mock_default_device_change() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Speakers", 44100));
        manager.add_device(MockDevice::new("Headphones", 48000));
        manager.add_device(MockDevice::new("USB DAC", 96000));

        // Initial default
        assert_eq!(manager.get_default_device().unwrap().name, "Speakers");

        // Change default to Headphones
        manager.set_default_device("Headphones");
        assert_eq!(manager.get_default_device().unwrap().name, "Headphones");

        // Change to USB DAC
        manager.set_default_device("USB DAC");
        assert_eq!(manager.get_default_device().unwrap().name, "USB DAC");
    }

    #[test]
    fn test_mock_default_device_removed() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Device 1", 44100));
        manager.add_device(MockDevice::new("Device 2", 48000));

        // Remove default device
        manager.remove_device("Device 1");

        // Should fall back to remaining device
        let default = manager.get_default_device().unwrap();
        assert_eq!(default.name, "Device 2");
    }

    #[test]
    fn test_mock_all_devices_removed() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Only Device", 44100));
        manager.remove_device("Only Device");

        // No default when no devices
        assert!(manager.get_default_device().is_none());
    }

    #[test]
    fn test_real_default_device_info() {
        let result = get_default_device(AudioBackend::Default);

        match result {
            Ok(device) => {
                eprintln!("Default device: {}", device.name);
                eprintln!("  Sample rate: {} Hz", device.sample_rate);
                eprintln!("  Channels: {}", device.channels);
                eprintln!("  Is default: {}", device.is_default);

                assert!(device.is_default);
                assert!(!device.name.is_empty());
            }
            Err(e) => {
                eprintln!("No default device (expected in CI): {}", e);
            }
        }
    }

    /// Test that playback adapts when default device changes
    #[test]
    #[ignore = "Requires audio hardware and OS settings change"]
    fn test_real_default_device_change() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        eprintln!("=== Default Device Change Test ===");
        eprintln!("This test requires:");
        eprintln!("  1. Open System Sound Settings");
        eprintln!("  2. Change default output device");
        eprintln!("  3. Verify the application detects the change");
    }
}

// ============================================================================
// 5. Multiple Device Enumeration Tests
// ============================================================================

mod multiple_device_enumeration_tests {
    use super::*;

    #[test]
    fn test_mock_enumerate_multiple_devices() {
        let mut manager = MockDeviceManager::new();

        manager.add_device(MockDevice::new("Built-in Speakers", 44100));
        manager.add_device(MockDevice::new("USB Headset", 48000));
        manager.add_device(MockDevice::high_end_dac("Audiophile DAC"));
        manager.add_device(MockDevice::new("HDMI Audio", 48000));

        let devices = manager.list_all_devices();
        assert_eq!(devices.len(), 4);

        // Verify each device has unique name
        let names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
        for (i, name) in names.iter().enumerate() {
            for (j, other) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(name, other, "Device names should be unique");
                }
            }
        }
    }

    #[test]
    fn test_mock_enumerate_with_disconnected_devices() {
        let mut manager = MockDeviceManager::new();

        let mut dev1 = MockDevice::new("Device 1", 44100);
        let dev2 = MockDevice::new("Device 2", 48000);
        let mut dev3 = MockDevice::new("Device 3", 96000);

        dev1.disconnect();
        dev3.disconnect();

        manager.add_device(dev1);
        manager.add_device(dev2);
        manager.add_device(dev3);

        // Only Device 2 should be available
        let available = manager.list_devices();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "Device 2");

        // All 3 should exist in full list
        assert_eq!(manager.list_all_devices().len(), 3);
    }

    #[test]
    fn test_real_enumerate_devices() {
        let result = list_devices(AudioBackend::Default);

        match result {
            Ok(devices) => {
                eprintln!("Found {} audio devices:", devices.len());
                for device in &devices {
                    eprintln!("  - {} ({}Hz, {} ch, default: {})",
                        device.name,
                        device.sample_rate,
                        device.channels,
                        device.is_default
                    );
                }

                // Verify basic invariants
                for device in &devices {
                    assert!(!device.name.is_empty());
                    assert!(device.sample_rate > 0);
                    assert!(device.channels > 0);
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate devices (expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_real_enumerate_with_capabilities() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        match result {
            Ok(devices) => {
                for device in &devices {
                    if let Some(caps) = &device.capabilities {
                        eprintln!("Device: {}", device.name);
                        eprintln!("  Sample rates: {:?}", caps.sample_rates);
                        eprintln!("  Bit depths: {:?}", caps.bit_depths);
                        eprintln!("  Max channels: {}", caps.max_channels);
                        eprintln!("  Exclusive: {}", caps.supports_exclusive);
                        eprintln!("  DSD: {}", caps.supports_dsd);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate with capabilities: {}", e);
            }
        }
    }

    #[test]
    fn test_concurrent_enumeration() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let result = list_devices(AudioBackend::Default);
                    (i, result.map(|d| d.len()))
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should either succeed with same count or fail consistently
        let success_counts: Vec<usize> = results
            .iter()
            .filter_map(|(_, r)| r.as_ref().ok().copied())
            .collect();

        if !success_counts.is_empty() {
            let first = success_counts[0];
            for count in &success_counts {
                assert_eq!(*count, first, "Concurrent enumeration should return consistent results");
            }
        }
    }
}

// ============================================================================
// 6. Device Switching Mid-Playback Tests
// ============================================================================

mod device_switching_tests {
    use super::*;

    #[test]
    fn test_real_switch_to_same_device() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let _original_device = playback.get_current_device();
                let original_backend = playback.get_current_backend();

                // Switch to same device (should succeed)
                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(_) => {
                        let new_backend = playback.get_current_backend();
                        assert_eq!(new_backend, original_backend);
                    }
                    Err(e) => {
                        eprintln!("Switch failed (may be expected): {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }

    #[test]
    fn test_real_switch_to_specific_device() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let devices = list_devices(AudioBackend::Default).unwrap_or_default();

                if devices.len() > 1 {
                    let target = &devices[1];
                    eprintln!("Switching to: {}", target.name);

                    match playback.switch_device(AudioBackend::Default, Some(target.name.clone())) {
                        Ok(_) => {
                            let current = playback.get_current_device();
                            assert_eq!(current, target.name);
                            eprintln!("Successfully switched to: {}", current);
                        }
                        Err(e) => {
                            eprintln!("Switch failed: {}", e);
                        }
                    }
                } else {
                    eprintln!("Only one device available, skipping switch test");
                }
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }

    #[test]
    fn test_real_switch_to_invalid_device() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let switch_result = playback.switch_device(
                    AudioBackend::Default,
                    Some("NonexistentDevice12345XYZ".to_string())
                );

                assert!(switch_result.is_err(), "Should fail for invalid device");
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }

    #[test]
    fn test_real_multiple_rapid_switches() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let mut success_count = 0;
                let mut fail_count = 0;

                for i in 0..5 {
                    match playback.switch_device(AudioBackend::Default, None) {
                        Ok(_) => {
                            success_count += 1;
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(e) => {
                            fail_count += 1;
                            eprintln!("Switch {} failed: {}", i, e);
                        }
                    }
                }

                eprintln!("Rapid switches: {} succeeded, {} failed", success_count, fail_count);
                assert!(success_count > 0, "At least some switches should succeed");
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }

    /// Test switching devices during active audio playback
    #[test]
    #[ignore = "Requires audio hardware and audio files"]
    fn test_real_switch_during_playback() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_switch.wav");
        create_test_wav(&wav_path, 10.0, 440.0, 44100).unwrap();

        let mut playback = match DesktopPlayback::new(PlaybackConfig::default()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
                return;
            }
        };

        let track = QueueTrack {
            id: "test-1".to_string(),
            path: wav_path.clone(),
            title: "Test".to_string(),
            artist: "Test Artist".to_string(),
            album: None,
            duration: Duration::from_secs(10),
            track_number: None,
            source: TrackSource::Single,
        };

        // Add track and start playback
        playback.send_command(PlaybackCommand::AddToQueue(track)).ok();
        playback.send_command(PlaybackCommand::Play).ok();

        thread::sleep(Duration::from_secs(2));

        // Switch device during playback
        let devices = list_devices(AudioBackend::Default).unwrap_or_default();
        if devices.len() > 1 {
            let target = &devices[1];
            eprintln!("Switching to {} during playback...", target.name);

            match playback.switch_device(AudioBackend::Default, Some(target.name.clone())) {
                Ok(_) => {
                    eprintln!("Switch successful");

                    // Verify playback continues
                    thread::sleep(Duration::from_secs(2));
                    let state = playback.get_state();
                    eprintln!("State after switch: {:?}", state);
                }
                Err(e) => {
                    eprintln!("Switch failed: {}", e);
                }
            }
        }

        playback.send_command(PlaybackCommand::Stop).ok();
    }
}

// ============================================================================
// 7. Sample Rate Mismatch Tests
// ============================================================================

mod sample_rate_mismatch_tests {
    use super::*;

    #[test]
    fn test_mock_sample_rate_mismatch_detection() {
        let device = MockDevice::new("48kHz Device", 48000);

        // File at 44.1kHz, device at 48kHz
        let file_rate = 44100;
        let device_rate = device.sample_rate;

        let needs_resampling = file_rate != device_rate;
        assert!(needs_resampling);

        // Calculate resampling ratio
        let ratio = device_rate as f64 / file_rate as f64;
        eprintln!("Resampling ratio: {:.6}", ratio);
        assert!((ratio - 1.088435).abs() < 0.001);
    }

    #[test]
    fn test_mock_sample_rate_match() {
        let device = MockDevice::new("44.1kHz Device", 44100);

        // File and device at same rate
        let file_rate = 44100;
        let device_rate = device.sample_rate;

        let needs_resampling = file_rate != device_rate;
        assert!(!needs_resampling);
    }

    #[test]
    fn test_real_audio_source_resampling() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_44k.wav");

        create_test_wav(&wav_path, 1.0, 440.0, 44100).unwrap();

        // Create source targeting 96kHz (will trigger resampling)
        let source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

        // Source should report target rate
        assert_eq!(source.sample_rate(), 96000);

        // File rate should be original
        assert_eq!(source.source_sample_rate(), 44100);
    }

    #[test]
    fn test_real_no_resampling_needed() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_48k.wav");

        create_test_wav(&wav_path, 1.0, 440.0, 48000).unwrap();

        // Create source targeting same rate
        let source = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");

        assert_eq!(source.sample_rate(), 48000);
        assert_eq!(source.source_sample_rate(), 48000);
    }

    #[test]
    fn test_real_common_rate_conversions() {
        let temp_dir = TempDir::new().unwrap();

        let conversions = vec![
            (44100, 48000, "CD to DAT"),
            (44100, 96000, "CD to high-res"),
            (48000, 96000, "DAT to high-res"),
            (96000, 44100, "high-res to CD"),
            (192000, 48000, "ultra high-res to DAT"),
        ];

        for (from_rate, to_rate, desc) in conversions {
            let wav_path = temp_dir.path().join(format!("test_{}_{}.wav", from_rate, to_rate));
            create_test_wav(&wav_path, 0.5, 1000.0, from_rate).unwrap();

            let source = LocalAudioSource::new(&wav_path, to_rate)
                .expect(&format!("Failed for {}", desc));

            assert_eq!(source.sample_rate(), to_rate, "{} output rate", desc);
            assert_eq!(source.source_sample_rate(), from_rate, "{} source rate", desc);

            eprintln!("{}: {}Hz -> {}Hz", desc, from_rate, to_rate);
        }
    }
}

// ============================================================================
// 8. Dynamic Sample Rate Change Tests
// ============================================================================

mod dynamic_sample_rate_tests {
    use super::*;

    #[test]
    fn test_mock_dynamic_rate_change() {
        let mut device = MockDevice::high_end_dac("Pro DAC");

        assert_eq!(device.sample_rate, 96000);

        // Simulate driver config change
        device.set_sample_rate(192000);
        assert_eq!(device.sample_rate, 192000);

        device.set_sample_rate(44100);
        assert_eq!(device.sample_rate, 44100);
    }

    #[test]
    fn test_mock_rate_change_during_operation() {
        let mut device = MockDevice::high_end_dac("Pro DAC");

        let rate_changes = vec![44100, 48000, 96000, 192000, 44100];

        for rate in rate_changes {
            device.set_sample_rate(rate);
            assert_eq!(device.sample_rate, rate);

            // Verify device stays available
            assert!(device.is_available());
        }
    }

    #[test]
    fn test_real_query_device_sample_rate() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(playback) => {
                let current_rate = playback.get_current_sample_rate();
                eprintln!("Current stream sample rate: {} Hz", current_rate);

                match playback.query_device_sample_rate() {
                    Ok(device_rate) => {
                        eprintln!("Device reported sample rate: {} Hz", device_rate);

                        // They should match after initialization
                        assert_eq!(current_rate, device_rate);
                    }
                    Err(e) => {
                        eprintln!("Could not query device rate: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not create playback: {}", e);
            }
        }
    }

    /// Test sample rate detection and adaptation
    #[test]
    #[ignore = "Requires audio hardware and driver config changes"]
    fn test_real_sample_rate_change_detection() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        eprintln!("=== Dynamic Sample Rate Change Test ===");
        eprintln!("This test requires:");
        eprintln!("  1. Create playback instance");
        eprintln!("  2. Note current sample rate");
        eprintln!("  3. Change device sample rate in driver settings (e.g., ASIO control panel)");
        eprintln!("  4. Call check_and_update_sample_rate()");
        eprintln!("  5. Verify rate change was detected");

        let result = DesktopPlayback::new(PlaybackConfig::default());

        if let Ok(mut playback) = result {
            let initial_rate = playback.get_current_sample_rate();
            eprintln!("Initial sample rate: {} Hz", initial_rate);

            // In a real test, you would:
            // 1. Pause here
            // 2. Change the device sample rate in driver settings
            // 3. Resume

            match playback.check_and_update_sample_rate() {
                Ok(changed) => {
                    if changed {
                        let new_rate = playback.get_current_sample_rate();
                        eprintln!("Sample rate changed to: {} Hz", new_rate);
                    } else {
                        eprintln!("Sample rate unchanged");
                    }
                }
                Err(e) => {
                    eprintln!("Error checking sample rate: {}", e);
                }
            }
        }
    }
}

// ============================================================================
// 9. Automatic Resampling Tests
// ============================================================================

mod automatic_resampling_tests {
    use super::*;

    #[test]
    fn test_resampling_preserves_duration() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_duration.wav");

        let expected_duration = 2.0;
        create_test_wav(&wav_path, expected_duration, 440.0, 44100).unwrap();

        // Upsample to 96kHz
        let source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

        let actual_duration = source.duration().as_secs_f64();
        eprintln!("Expected: {:.3}s, Actual: {:.3}s", expected_duration, actual_duration);

        // Allow 5% tolerance
        assert!((actual_duration - expected_duration as f64).abs() < 0.1,
            "Duration should be preserved");
    }

    #[test]
    fn test_resampling_produces_valid_samples() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_samples.wav");

        create_test_wav(&wav_path, 1.0, 440.0, 44100).unwrap();

        let mut source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

        // Read some samples
        let mut buffer = vec![0.0f32; 9600]; // 0.05s at 96kHz stereo
        let samples_read = source.read_samples(&mut buffer).expect("Failed to read");

        assert!(samples_read > 0);

        // All samples should be in valid range
        for sample in &buffer[..samples_read] {
            assert!(sample.abs() <= 1.0, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_resampling_extreme_ratios() {
        let temp_dir = TempDir::new().unwrap();

        // Extreme upsampling: 22050 -> 192000 (8.7x)
        let wav_path = temp_dir.path().join("test_extreme_up.wav");
        create_test_wav(&wav_path, 0.5, 440.0, 22050).unwrap();

        let source = LocalAudioSource::new(&wav_path, 192000).expect("Failed upsampling");
        assert_eq!(source.sample_rate(), 192000);

        // Extreme downsampling: 192000 -> 22050 (0.11x)
        let wav_path2 = temp_dir.path().join("test_extreme_down.wav");
        create_test_wav(&wav_path2, 0.5, 1000.0, 192000).unwrap();

        let source2 = LocalAudioSource::new(&wav_path2, 22050).expect("Failed downsampling");
        assert_eq!(source2.sample_rate(), 22050);
    }
}

// ============================================================================
// 10. Sample Rate Negotiation Tests
// ============================================================================

mod sample_rate_negotiation_tests {
    use super::*;

    #[test]
    fn test_mock_device_supports_rate() {
        let device = MockDevice::new("Standard Device", 48000);

        let supported = &device.capabilities.sample_rates;

        assert!(supported.contains(&44100));
        assert!(supported.contains(&48000));
        assert!(supported.contains(&96000));
    }

    #[test]
    fn test_mock_find_best_rate() {
        let device = MockDevice::high_end_dac("Pro DAC");
        let caps = &device.capabilities;

        // Find closest supported rate to 88200
        let target = 88200u32;
        let best = caps.sample_rates.iter()
            .min_by_key(|&&rate| (rate as i64 - target as i64).abs())
            .copied()
            .unwrap_or(44100);

        assert_eq!(best, 88200);

        // Find closest to unsupported rate like 50000
        let target = 50000u32;
        let best = caps.sample_rates.iter()
            .min_by_key(|&&rate| (rate as i64 - target as i64).abs())
            .copied()
            .unwrap_or(44100);

        assert_eq!(best, 48000);
    }

    #[test]
    fn test_real_device_supported_rates() {
        let result = get_default_device_with_capabilities(AudioBackend::Default, true);

        match result {
            Ok(device) => {
                if let Some(caps) = &device.capabilities {
                    eprintln!("Device {} supports:", device.name);
                    eprintln!("  Sample rates: {:?}", caps.sample_rates);

                    // All supported rates should be reasonable
                    for rate in &caps.sample_rates {
                        assert!(*rate >= 8000);
                        assert!(*rate <= 768000);
                    }

                    // Should include at least one standard rate
                    let has_standard = caps.sample_rates.iter()
                        .any(|r| STANDARD_SAMPLE_RATES.contains(r));
                    assert!(has_standard, "Device should support at least one standard rate");
                }
            }
            Err(e) => {
                eprintln!("Could not get device info: {}", e);
            }
        }
    }

    #[test]
    fn test_real_rate_range() {
        let result = get_default_device(AudioBackend::Default);

        match result {
            Ok(device) => {
                if let Some((min, max)) = device.sample_rate_range {
                    eprintln!("Device {} rate range: {} - {} Hz", device.name, min, max);

                    assert!(min <= max);
                    assert!(min >= 8000);
                    assert!(max <= 768000);
                }
            }
            Err(e) => {
                eprintln!("Could not get device: {}", e);
            }
        }
    }
}

// ============================================================================
// 11. Integration Smoke Tests
// ============================================================================

mod integration_smoke_tests {
    use super::*;

    /// Complete device handling workflow test
    #[test]
    fn test_complete_device_workflow() {
        eprintln!("=== Complete Device Workflow Test ===\n");

        // Step 1: List backends
        eprintln!("Step 1: List available backends");
        let backends = list_available_backends();
        eprintln!("  Found {} backends: {:?}", backends.len(), backends);
        assert!(!backends.is_empty());

        // Step 2: List devices
        eprintln!("\nStep 2: Enumerate devices");
        match list_devices(AudioBackend::Default) {
            Ok(devices) => {
                eprintln!("  Found {} devices", devices.len());
                for d in &devices {
                    eprintln!("    - {} ({}Hz, default: {})", d.name, d.sample_rate, d.is_default);
                }
            }
            Err(e) => {
                eprintln!("  Could not list devices: {}", e);
            }
        }

        // Step 3: Create playback
        eprintln!("\nStep 3: Create playback instance");
        match DesktopPlayback::new(PlaybackConfig::default()) {
            Ok(playback) => {
                eprintln!("  Created playback on: {}", playback.get_current_device());
                eprintln!("  Sample rate: {} Hz", playback.get_current_sample_rate());
                eprintln!("  Backend: {:?}", playback.get_current_backend());
            }
            Err(e) => {
                eprintln!("  Could not create playback: {}", e);
            }
        }

        eprintln!("\n=== Workflow Complete ===");
    }

    #[test]
    fn test_mock_complete_workflow() {
        let mut manager = MockDeviceManager::new();

        // Add various devices
        manager.add_device(MockDevice::new("Built-in Speakers", 44100));
        manager.add_device(MockDevice::high_end_dac("External DAC"));
        manager.add_device(MockDevice::new("Headphones", 48000));

        // List all devices
        let all = manager.list_all_devices();
        assert_eq!(all.len(), 3);

        // Get default
        let default = manager.get_default_device().unwrap();
        assert_eq!(default.name, "Built-in Speakers");

        // Change default
        manager.set_default_device("External DAC");
        let default = manager.get_default_device().unwrap();
        assert_eq!(default.name, "External DAC");

        // Simulate device disconnect
        manager.get_device_mut("External DAC").unwrap().disconnect();
        let available = manager.list_devices();
        assert_eq!(available.len(), 2);

        // Reconnect
        manager.get_device_mut("External DAC").unwrap().reconnect();
        let available = manager.list_devices();
        assert_eq!(available.len(), 3);
    }
}

// ============================================================================
// 12. Error Handling Tests
// ============================================================================

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_device_error_types() {
        // Test error construction
        let err1 = DeviceError::DeviceNotFound("Test Device".to_string());
        assert!(err1.to_string().contains("Test Device"));

        let err2 = DeviceError::BackendUnavailable("ASIO");
        assert!(err2.to_string().contains("ASIO"));

        let err3 = DeviceError::EnumerationFailed("Some error".to_string());
        assert!(err3.to_string().contains("Some error"));
    }

    #[test]
    fn test_find_nonexistent_device_error() {
        let result = find_device_by_name(AudioBackend::Default, "NonexistentDevice12345");

        assert!(result.is_err());
        match result {
            Err(DeviceError::DeviceNotFound(name)) => {
                assert!(name.contains("Nonexistent"));
            }
            Err(other) => {
                eprintln!("Unexpected error type: {:?}", other);
            }
            Ok(_) => {
                panic!("Should have failed");
            }
        }
    }

    #[test]
    fn test_graceful_degradation_no_devices() {
        // This tests the code path when enumeration succeeds but returns empty
        let result = list_devices(AudioBackend::Default);

        match result {
            Ok(devices) => {
                eprintln!("System has {} devices", devices.len());
            }
            Err(e) => {
                eprintln!("Enumeration failed (may be expected in CI): {}", e);
            }
        }
        // Test passes if no panic occurs
    }

    #[test]
    fn test_mock_error_state_handling() {
        let mut device = MockDevice::new("Error Device", 48000);
        device.state = MockDeviceState::Error("Driver crashed".to_string());

        assert!(!device.is_available());

        // Recovery
        device.reconnect();
        assert!(device.is_available());
    }
}

// ============================================================================
// 13. Concurrent Access Tests
// ============================================================================

mod concurrent_access_tests {
    use super::*;

    #[test]
    fn test_concurrent_device_queries() {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                thread::spawn(move || {
                    for _ in 0..10 {
                        let _ = list_devices(AudioBackend::Default);
                        let _ = get_default_device(AudioBackend::Default);
                        let _ = list_available_backends();
                    }
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }

    #[test]
    fn test_concurrent_mock_device_access() {
        use std::sync::Mutex;

        let manager = Arc::new(Mutex::new(MockDeviceManager::new()));

        // Add initial device
        {
            let mut m = manager.lock().unwrap();
            m.add_device(MockDevice::new("Shared Device", 48000));
        }

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let manager = Arc::clone(&manager);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let m = manager.lock().unwrap();
                        let _ = m.list_devices();
                        let _ = m.get_default_device();
                        drop(m);

                        // Small sleep to allow contention
                        thread::sleep(Duration::from_micros(10));
                    }
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }
}

// ============================================================================
// 14. Property-Based Tests
// ============================================================================

mod property_tests {
    use super::*;

    #[test]
    fn test_device_invariants() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                // Name should not be empty
                assert!(!device.name.is_empty());

                // Sample rate should be positive and reasonable
                assert!(device.sample_rate >= 8000);
                assert!(device.sample_rate <= 768000);

                // Channels should be positive and reasonable
                assert!(device.channels >= 1);
                assert!(device.channels <= 32);

                // If has capabilities, validate them
                if let Some(caps) = &device.capabilities {
                    assert!(!caps.sample_rates.is_empty());
                    assert!(!caps.bit_depths.is_empty());
                    assert!(caps.max_channels >= 1);

                    // Sample rates should be sorted
                    for i in 1..caps.sample_rates.len() {
                        assert!(caps.sample_rates[i-1] <= caps.sample_rates[i]);
                    }
                }
            }
        }
    }

    #[test]
    fn test_mock_device_state_transitions() {
        let mut device = MockDevice::new("State Test", 48000);

        let transitions = vec![
            (MockDeviceState::Connected, true),
            (MockDeviceState::Disconnected, false),
            (MockDeviceState::Reconnecting, false),
            (MockDeviceState::Error("test".to_string()), false),
            (MockDeviceState::Connected, true),
        ];

        for (state, expected_available) in transitions {
            device.state = state.clone();
            assert_eq!(device.is_available(), expected_available,
                "State {:?} should have is_available={}", state, expected_available);
        }
    }

    #[test]
    fn test_resampling_ratio_accuracy() {
        let test_cases = vec![
            (44100, 48000),
            (44100, 96000),
            (48000, 96000),
            (96000, 192000),
            (192000, 44100),
        ];

        for (from, to) in test_cases {
            let ratio = to as f64 / from as f64;

            // Verify ratio is reasonable
            assert!(ratio > 0.0);
            assert!(ratio < 10.0);

            // Inverse ratio should equal 1.0 when multiplied
            let inverse = from as f64 / to as f64;
            let product = ratio * inverse;
            assert!((product - 1.0).abs() < 1e-10);
        }
    }
}

// ============================================================================
// 15. Manual Test Guides
// ============================================================================

/// This module provides documentation for manual hardware tests
#[cfg(test)]
mod manual_test_guides {
    #[test]
    #[ignore = "Documentation test"]
    fn print_device_hotplug_test_guide() {
        eprintln!("\n=== DEVICE HOT-PLUG MANUAL TEST GUIDE ===\n");

        eprintln!("1. USB AUDIO DEVICE DISCONNECTION:");
        eprintln!("   a. Start playback on USB audio device");
        eprintln!("   b. Physically unplug the USB device");
        eprintln!("   c. Verify: Error event is emitted");
        eprintln!("   d. Verify: Playback stops gracefully or switches to fallback");
        eprintln!("   e. Verify: No crash or hang\n");

        eprintln!("2. USB AUDIO DEVICE RECONNECTION:");
        eprintln!("   a. After disconnection, replug the USB device");
        eprintln!("   b. Re-enumerate devices");
        eprintln!("   c. Verify: Device appears in list");
        eprintln!("   d. Verify: Can switch to the device");
        eprintln!("   e. Verify: Playback works on reconnected device\n");

        eprintln!("3. DEFAULT DEVICE CHANGE:");
        eprintln!("   a. Start playback on current default device");
        eprintln!("   b. Change default in OS settings");
        eprintln!("   c. Verify: Application detects change (or handles gracefully)");
        eprintln!("   d. Verify: Manual switch to new default works\n");

        eprintln!("4. BLUETOOTH DEVICE:");
        eprintln!("   a. Pair and connect Bluetooth headphones");
        eprintln!("   b. Verify: Device appears in list");
        eprintln!("   c. Switch to Bluetooth device");
        eprintln!("   d. Start playback");
        eprintln!("   e. Turn off Bluetooth headphones");
        eprintln!("   f. Verify: Error handling\n");
    }

    #[test]
    #[ignore = "Documentation test"]
    fn print_sample_rate_test_guide() {
        eprintln!("\n=== SAMPLE RATE MANUAL TEST GUIDE ===\n");

        eprintln!("1. ASIO SAMPLE RATE CHANGE:");
        eprintln!("   a. Start playback using ASIO backend");
        eprintln!("   b. Open ASIO control panel");
        eprintln!("   c. Change sample rate (e.g., 44100 -> 96000)");
        eprintln!("   d. Verify: Application detects the change");
        eprintln!("   e. Verify: Audio source is reloaded");
        eprintln!("   f. Verify: Playback continues at correct speed\n");

        eprintln!("2. WASAPI SAMPLE RATE:");
        eprintln!("   a. Open Windows Sound Settings");
        eprintln!("   b. Select output device -> Properties -> Advanced");
        eprintln!("   c. Change sample rate");
        eprintln!("   d. Restart playback");
        eprintln!("   e. Verify: Uses new sample rate\n");

        eprintln!("3. DEVICE WITH FIXED RATE:");
        eprintln!("   a. Use device that only supports one rate (e.g., some USB DACs)");
        eprintln!("   b. Play files at different sample rates");
        eprintln!("   c. Verify: Resampling occurs automatically");
        eprintln!("   d. Verify: No pitch shift or speed change\n");
    }
}
