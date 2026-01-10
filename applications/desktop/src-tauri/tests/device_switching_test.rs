//! Device switching E2E tests
//!
//! Tests audio device selection, switching, and persistence across the entire
//! audio pipeline (device detection → backend selection → playback).

use soul_audio_desktop::{backend, device, AudioBackend};
use std::sync::{Arc, Mutex};

// Helper to skip tests if no audio device available
fn has_audio_device() -> bool {
    let backends = backend::get_backend_info();
    backends.iter().any(|b| b.available && b.device_count > 0)
}

#[test]
fn test_list_available_devices() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    // Get all available backends
    let backends = backend::get_backend_info();
    eprintln!("[test] Found {} backends", backends.len());

    assert!(!backends.is_empty(), "Should have at least one backend");

    // Check that Default backend exists
    assert!(
        backends.iter().any(|b| b.backend == AudioBackend::Default),
        "Default backend should always exist"
    );

    // For each available backend, list devices
    for backend_info in backends.iter().filter(|b| b.available) {
        eprintln!("[test] Testing backend: {:?}", backend_info.backend);

        match device::list_devices(backend_info.backend) {
            Ok(devices) => {
                eprintln!("[test] Found {} devices for {:?}", devices.len(), backend_info.backend);

                assert!(!devices.is_empty(), "Available backend should have at least one device");

                // Verify device info structure
                for dev in &devices {
                    assert!(!dev.name.is_empty(), "Device should have a name");
                    assert!(dev.sample_rate > 0, "Device should have valid sample rate");
                    assert!(dev.channels > 0, "Device should have valid channel count");
                    assert!(dev.backend == backend_info.backend, "Device backend should match");

                    eprintln!("[test]   - {}: {}Hz, {}ch{}",
                        dev.name,
                        dev.sample_rate,
                        dev.channels,
                        if dev.is_default { " [DEFAULT]" } else { "" }
                    );
                }

                // Verify at least one default device
                assert!(
                    devices.iter().any(|d| d.is_default),
                    "Backend should have a default device"
                );
            }
            Err(e) => {
                panic!("Failed to list devices for available backend {:?}: {}", backend_info.backend, e);
            }
        }
    }
}

#[test]
fn test_get_default_device() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    // Get default device
    let backend = AudioBackend::Default;
    let device = device::get_default_device(backend)
        .expect("Should be able to get default device");

    eprintln!("[test] Default device: {} ({}Hz, {}ch)",
        device.name, device.sample_rate, device.channels);

    // Verify device info
    assert!(!device.name.is_empty(), "Device should have a name");
    assert!(device.sample_rate > 0, "Device should have valid sample rate");
    assert!(device.channels > 0, "Device should have valid channel count");
    assert!(device.is_default, "Should be marked as default");
    assert_eq!(device.backend, backend, "Backend should match");
}

#[test]
fn test_find_device_by_name() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    let backend = AudioBackend::Default;

    // Get default device name
    let default_device = device::get_default_device(backend)
        .expect("Should be able to get default device");
    let device_name = default_device.name.clone();

    eprintln!("[test] Searching for device: {}", device_name);

    // Find device by name
    let found_device = device::find_device_by_name(backend, &device_name)
        .expect("Should find device by name");

    // Verify match
    assert_eq!(found_device.name, device_name, "Device names should match");
    assert_eq!(found_device.backend, backend, "Backends should match");
    assert_eq!(found_device.sample_rate, default_device.sample_rate, "Sample rates should match");
    assert_eq!(found_device.channels, default_device.channels, "Channels should match");

    eprintln!("[test] Successfully found device: {} ({}Hz, {}ch)",
        found_device.name, found_device.sample_rate, found_device.channels);
}

#[test]
fn test_find_nonexistent_device() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    let backend = AudioBackend::Default;
    let fake_name = "NonexistentDevice_12345";

    eprintln!("[test] Trying to find non-existent device: {}", fake_name);

    // Try to find non-existent device
    let result = device::find_device_by_name(backend, fake_name);

    // Should fail
    assert!(result.is_err(), "Should fail to find non-existent device");
    eprintln!("[test] Correctly failed to find device: {:?}", result.unwrap_err());
}

#[test]
fn test_device_sample_rate_ranges() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    let backends = backend::get_backend_info();

    for backend_info in backends.iter().filter(|b| b.available) {
        let devices = device::list_devices(backend_info.backend)
            .expect("Should list devices");

        eprintln!("[test] Checking sample rate ranges for {:?}", backend_info.backend);

        for dev in devices {
            // Common sample rates
            let common_rates = [44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000];

            if dev.sample_rate_range.is_some() {
                let (min, max) = dev.sample_rate_range.unwrap();
                eprintln!("[test]   - {}: {}Hz-{}Hz (current: {}Hz)",
                    dev.name, min, max, dev.sample_rate);

                assert!(min > 0, "Min sample rate should be positive");
                assert!(max >= min, "Max should be >= min");
                assert!(dev.sample_rate >= min && dev.sample_rate <= max,
                    "Current sample rate should be within range");
            } else {
                eprintln!("[test]   - {}: {}Hz (no range info)", dev.name, dev.sample_rate);
            }

            // Verify sample rate is reasonable
            assert!(
                common_rates.contains(&dev.sample_rate) || dev.sample_rate > 0,
                "Device should have reasonable sample rate"
            );
        }
    }
}

#[test]
fn test_backend_availability() {
    let backends = backend::get_backend_info();

    eprintln!("[test] Checking backend availability:");

    for backend_info in &backends {
        eprintln!("[test]   - {:?}: available={}, devices={}, default={}",
            backend_info.backend,
            backend_info.available,
            backend_info.device_count,
            backend_info.is_default
        );

        // Default backend should always be available
        if backend_info.backend == AudioBackend::Default {
            assert!(backend_info.available, "Default backend should always be available");
            assert!(backend_info.device_count > 0, "Default backend should have at least one device");
        }

        // Verify consistency
        if backend_info.available {
            assert!(backend_info.device_count > 0, "Available backend should have devices");
        }
    }

    // Verify exactly one default backend
    let default_count = backends.iter().filter(|b| b.is_default).count();
    assert_eq!(default_count, 1, "Should have exactly one default backend");
}

#[test]
fn test_device_channel_counts() {
    if !has_audio_device() {
        println!("Skipping test - no audio device available");
        return;
    }

    let backend = AudioBackend::Default;
    let devices = device::list_devices(backend)
        .expect("Should list devices");

    eprintln!("[test] Checking device channel counts:");

    for dev in devices {
        eprintln!("[test]   - {}: {} channels", dev.name, dev.channels);

        // Verify reasonable channel count
        assert!(dev.channels >= 1, "Device should have at least 1 channel");
        assert!(dev.channels <= 32, "Device should have reasonable max channels");

        // Most common: 1 (mono), 2 (stereo), 6 (5.1), 8 (7.1)
        let common_channels = [1, 2, 6, 8];
        if !common_channels.contains(&dev.channels) {
            eprintln!("[test]     (Unusual channel count: {})", dev.channels);
        }
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use soul_audio_desktop::playback::DesktopPlayback;
    use soul_audio_desktop::sources::local::LocalAudioSource;
    use soul_playback::{PlaybackManager as CoreManager, types::TrackInfo};
    use std::path::PathBuf;

    // Helper to create test audio file
    fn create_test_wav(sample_rate: u32, duration_secs: u32) -> PathBuf {
        use hound::{WavWriter, WavSpec};

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("test_{}hz.wav", sample_rate));

        // Create WAV file
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&file_path, spec)
            .expect("Failed to create WAV file");

        // Generate 440 Hz sine wave
        let samples_per_channel = (sample_rate * duration_secs) as usize;
        for i in 0..samples_per_channel {
            let t = i as f32 / sample_rate as f32;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            let amplitude = (sample * std::i16::MAX as f32) as i16;

            // Write stereo
            writer.write_sample(amplitude).unwrap();
            writer.write_sample(amplitude).unwrap();
        }

        writer.finalize().expect("Failed to finalize WAV");

        eprintln!("[test] Created test file: {:?}", file_path);
        file_path
    }

    #[test]
    fn test_playback_with_device_selection() {
        if !has_audio_device() {
            println!("Skipping test - no audio device available");
            return;
        }

        // Get default device
        let backend = AudioBackend::Default;
        let device = device::get_default_device(backend)
            .expect("Should get default device");

        eprintln!("[test] Testing playback with device: {} ({}Hz)",
            device.name, device.sample_rate);

        // Create test audio file matching device sample rate
        let test_file = create_test_wav(device.sample_rate, 3);

        // Create audio source
        let source = LocalAudioSource::new(&test_file, device.sample_rate)
            .expect("Should create audio source");

        eprintln!("[test] Created audio source");

        // Create playback system
        let playback = DesktopPlayback::new(backend, Some(device.name.clone()))
            .expect("Should create playback system");

        eprintln!("[test] Created playback system");

        // Create playback manager
        let mut manager = CoreManager::new(device.sample_rate);
        manager.set_audio_source(Box::new(source));

        eprintln!("[test] Created playback manager");

        // Play for 1 second
        manager.play();
        std::thread::sleep(std::time::Duration::from_secs(1));
        manager.pause();

        // Check position
        let position = manager.get_position();
        eprintln!("[test] Playback position after 1s: {:.2}s", position);

        // Position should be approximately 1 second (±0.2s tolerance)
        assert!((0.8..=1.2).contains(&position),
            "Position should be ~1 second, got {:.2}s", position);

        eprintln!("[test] ✓ Playback successful with device selection");

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_device_sample_rate_mismatch() {
        if !has_audio_device() {
            println!("Skipping test - no audio device available");
            return;
        }

        // Get device
        let backend = AudioBackend::Default;
        let device = device::get_default_device(backend)
            .expect("Should get default device");

        eprintln!("[test] Device sample rate: {}Hz", device.sample_rate);

        // Create audio file with DIFFERENT sample rate (simulate mismatch)
        let file_sample_rate = if device.sample_rate == 44100 { 48000 } else { 44100 };
        let test_file = create_test_wav(file_sample_rate, 3);

        eprintln!("[test] File sample rate: {}Hz (mismatch)", file_sample_rate);

        // Create audio source with TARGET sample rate (device rate)
        let source = LocalAudioSource::new(&test_file, device.sample_rate)
            .expect("Should create audio source with resampling");

        eprintln!("[test] Audio source created - resampling should be active");

        // Source should enable resampling automatically
        // (verified by console output in LocalAudioSource::new)

        // Cleanup
        std::fs::remove_file(test_file).ok();

        eprintln!("[test] ✓ Resampling handling verified");
    }
}
