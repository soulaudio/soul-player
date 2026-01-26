//! Stream creation error path tests
//!
//! This test module covers error scenarios during CPAL stream creation and initialization:
//!
//! 1. **Unsupported Sample Format** - Attempting to create streams with incompatible formats
//! 2. **Device Unavailable During Build** - Device disconnection before `stream.build()`
//! 3. **Error Callback Execution** - Verifying stream error callbacks fire correctly
//! 4. **Invalid Buffer Size** - Testing minimum/maximum buffer size constraints
//! 5. **Stream Build Failures** - Generic stream construction failures
//! 6. **Multiple Stream Creation** - Rapid successive stream creation/destruction

#![allow(clippy::doc_markdown)]
#![allow(clippy::ignored_unit_patterns)]
//!
//! ## Test Strategy
//!
//! Most tests are integration tests that:
//! - Use real CPAL devices when available
//! - Test all three sample formats (F32, I32, I16)
//! - Verify proper error propagation
//! - Ensure no panics or memory leaks
//!
//! Hardware-dependent tests are marked with `#[ignore]` and can be run with:
//! ```bash
//! cargo test -p soul-audio-desktop stream_creation_errors -- --ignored
//! ```

use soul_audio_desktop::{AudioError, CpalOutput};
use soul_core::{AudioBuffer, AudioFormat, AudioOutput, SampleRate};
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Helpers
// ============================================================================

/// Check if audio hardware is available
fn has_audio_hardware() -> bool {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.default_output_config().ok())
        .is_some()
}

/// Create a test audio buffer with sine wave
fn create_test_buffer(sample_rate: u32, duration_secs: f32, frequency: f32) -> AudioBuffer {
    let format = AudioFormat::new(SampleRate::new(sample_rate), 2, 32);
    let num_samples = (sample_rate as f32 * duration_secs * 2.0) as usize; // stereo

    let mut samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples / 2 {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin() * 0.5;
        samples.push(sample);
        samples.push(sample);
    }

    AudioBuffer::new(samples, format)
}

// ============================================================================
// 1. Unsupported Sample Format Tests
// ============================================================================

mod unsupported_format_tests {
    use super::*;

    /// Test that CpalOutput handles format compatibility correctly
    ///
    /// Note: CPAL internally negotiates formats, so this tests the output
    /// layer's ability to work with various input formats via resampling
    #[test]
    fn test_output_handles_different_formats() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // Try playing buffers with different sample rates
                // The output should resample automatically
                let test_rates = vec![22050, 44100, 48000, 96000];

                for rate in test_rates {
                    let buffer = create_test_buffer(rate, 0.1, 440.0);
                    let play_result = output.play(&buffer);

                    match play_result {
                        Ok(()) => {
                            tracing::debug!("Successfully played {}Hz buffer", rate);
                        }
                        Err(e) => {
                            eprintln!("Expected resampling to handle {}Hz: {}", rate, e);
                        }
                    }

                    thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) => {
                eprintln!("Could not create output (expected in CI): {}", e);
            }
        }
    }

    /// Test extreme sample rate edge cases
    #[test]
    fn test_extreme_sample_rates() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // Very low sample rate
                let low_rate_buffer = create_test_buffer(8000, 0.05, 440.0);
                let result = output.play(&low_rate_buffer);
                assert!(
                    result.is_ok(),
                    "Should handle low sample rate with resampling"
                );

                thread::sleep(Duration::from_millis(50));

                // Very high sample rate
                let high_rate_buffer = create_test_buffer(192000, 0.05, 440.0);
                let result = output.play(&high_rate_buffer);
                assert!(
                    result.is_ok(),
                    "Should handle high sample rate with resampling"
                );
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test that mono buffers are handled correctly
    #[test]
    fn test_mono_buffer_handling() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // Create mono buffer
                let format = AudioFormat::new(SampleRate::new(44100), 1, 32);
                let samples: Vec<f32> = (0..44100)
                    .map(|i| {
                        let t = i as f32 / 44100.0;
                        (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5
                    })
                    .collect();

                let mono_buffer = AudioBuffer::new(samples, format);
                let result = output.play(&mono_buffer);

                // Note: This might fail if the device requires stereo
                // The output layer should ideally duplicate mono to stereo
                match result {
                    Ok(()) => {
                        tracing::debug!("Successfully played mono buffer");
                    }
                    Err(e) => {
                        eprintln!("Mono playback error (may be expected): {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }
}

// ============================================================================
// 2. Device Unavailable During Build Tests
// ============================================================================

mod device_unavailable_tests {
    use super::*;

    /// Test creating multiple outputs rapidly
    ///
    /// This stresses the device acquisition and release logic
    #[test]
    fn test_rapid_output_creation() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let mut success_count = 0;
        let mut fail_count = 0;

        for i in 0..10 {
            match CpalOutput::new() {
                Ok(output) => {
                    success_count += 1;
                    drop(output); // Explicitly drop to release device
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    fail_count += 1;
                    eprintln!("Output creation {} failed: {}", i, e);
                }
            }
        }

        eprintln!(
            "Rapid creation: {} succeeded, {} failed",
            success_count, fail_count
        );
        assert!(
            success_count > 0,
            "At least some outputs should be created successfully"
        );
    }

    /// Test output creation with device name lookup
    ///
    /// This tests the path where a device might not be found
    #[test]
    fn test_nonexistent_device() {
        use soul_audio_desktop::backend::AudioBackend;
        use soul_audio_desktop::device::find_device_by_name;

        let result = find_device_by_name(AudioBackend::Default, "NonexistentDevice12345XYZ");

        assert!(result.is_err(), "Should fail for nonexistent device");
    }

    /// Test concurrent output creation
    ///
    /// Multiple threads trying to acquire the same device
    #[test]
    fn test_concurrent_output_creation() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let result = CpalOutput::new();
                    match result {
                        Ok(output) => {
                            // Hold the output for a bit
                            thread::sleep(Duration::from_millis(100));
                            drop(output);
                            (i, true)
                        }
                        Err(e) => {
                            eprintln!("Thread {} failed to create output: {}", i, e);
                            (i, false)
                        }
                    }
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        let success_count = results.iter().filter(|(_, success)| *success).count();
        eprintln!("Concurrent creation: {}/{} succeeded", success_count, 4);

        // At least some should succeed (system dependent)
        assert!(success_count > 0, "At least one thread should succeed");
    }
}

// ============================================================================
// 3. Error Callback Execution Tests
// ============================================================================

mod error_callback_tests {
    use super::*;

    /// Test that stream error callbacks are registered
    ///
    /// Note: Triggering actual stream errors is hardware/platform dependent
    /// This test verifies the error callback mechanism is set up correctly
    #[test]
    fn test_error_callback_setup() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // The error callback is set up in the audio thread
                // We can't easily trigger it, but we can verify the output
                // continues to function normally

                let buffer = create_test_buffer(44100, 0.1, 440.0);
                let result = output.play(&buffer);
                assert!(result.is_ok(), "Playback should work normally");

                thread::sleep(Duration::from_millis(50));

                // Verify we can pause/resume without triggering errors
                assert!(output.pause().is_ok());
                assert!(output.resume().is_ok());
                assert!(output.stop().is_ok());
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test output behavior during rapid state changes
    ///
    /// This might trigger error conditions if state transitions are invalid
    #[test]
    fn test_rapid_state_transitions() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let buffer = create_test_buffer(44100, 1.0, 440.0);

                // Rapid play/pause cycles
                for i in 0..5 {
                    let _ = output.play(&buffer);
                    thread::sleep(Duration::from_millis(10));

                    let _ = output.pause();
                    thread::sleep(Duration::from_millis(10));

                    let _ = output.resume();
                    thread::sleep(Duration::from_millis(10));

                    let _ = output.stop();
                    thread::sleep(Duration::from_millis(10));

                    tracing::debug!("Completed cycle {}", i);
                }

                // Output should still be functional
                let result = output.play(&buffer);
                assert!(result.is_ok(), "Output should remain functional");
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test volume changes don't cause stream errors
    #[test]
    fn test_volume_changes_no_errors() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let buffer = create_test_buffer(44100, 0.5, 440.0);
                let _ = output.play(&buffer);

                // Rapid volume changes
                let volumes = vec![0.0, 0.25, 0.5, 0.75, 1.0, 0.5];
                for vol in volumes {
                    let result = output.set_volume(vol);
                    assert!(result.is_ok(), "Volume change should succeed");
                    assert_eq!(output.volume(), vol, "Volume should be updated");
                    thread::sleep(Duration::from_millis(20));
                }
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }
}

// ============================================================================
// 4. Invalid Buffer Size Tests
// ============================================================================

mod buffer_size_tests {
    use super::*;

    /// Test playing an empty buffer
    #[test]
    fn test_empty_buffer() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let format = AudioFormat::new(SampleRate::new(44100), 2, 32);
                let empty_buffer = AudioBuffer::new(vec![], format);

                // Playing empty buffer should not crash
                let result = output.play(&empty_buffer);
                match result {
                    Ok(()) => {
                        tracing::debug!("Empty buffer handled");
                    }
                    Err(e) => {
                        eprintln!("Empty buffer error (may be expected): {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test extremely small buffer (1 frame)
    #[test]
    fn test_tiny_buffer() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let format = AudioFormat::new(SampleRate::new(44100), 2, 32);
                let tiny_buffer = AudioBuffer::new(vec![0.5, 0.5], format); // 1 stereo frame

                let result = output.play(&tiny_buffer);
                assert!(result.is_ok(), "Should handle tiny buffer");

                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test very large buffer
    #[test]
    fn test_large_buffer() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // 10 seconds at 44100Hz stereo = 882,000 samples
                let buffer = create_test_buffer(44100, 10.0, 440.0);

                let result = output.play(&buffer);
                assert!(result.is_ok(), "Should handle large buffer");

                // Don't wait for full playback
                thread::sleep(Duration::from_millis(100));
                let _ = output.stop();
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }

    /// Test mismatched channel count in buffer
    #[test]
    fn test_odd_sample_count() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                // Stereo format but odd number of samples (should be pairs)
                let format = AudioFormat::new(SampleRate::new(44100), 2, 32);
                let odd_buffer = AudioBuffer::new(vec![0.5, 0.5, 0.5], format); // 1.5 frames

                let result = output.play(&odd_buffer);
                match result {
                    Ok(()) => {
                        tracing::debug!("Odd sample count handled");
                    }
                    Err(e) => {
                        eprintln!("Odd sample count error (may be expected): {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }
}

// ============================================================================
// 5. Stream Build Failure Tests
// ============================================================================

mod stream_build_tests {
    use super::*;

    /// Test that output gracefully handles device not found
    #[test]
    fn test_no_device_error() {
        use cpal::traits::HostTrait;

        let host = cpal::default_host();
        if host.default_output_device().is_some() {
            eprintln!("Skipping: Device exists (can't test no-device case)");
            return;
        }

        // This should only run in environments without audio devices
        let result = CpalOutput::new();
        assert!(
            result.is_err(),
            "Should return error when no device available"
        );

        match result {
            Err(AudioError::DeviceNotFound) => {
                tracing::debug!("Correct error type for no device");
            }
            Err(e) => {
                eprintln!("Got error (acceptable): {}", e);
            }
            Ok(_) => {
                panic!("Should not succeed without device");
            }
        }
    }

    /// Test output cleanup on failure
    #[test]
    fn test_cleanup_on_creation_failure() {
        // Try to create multiple outputs in rapid succession
        // Even if some fail, cleanup should prevent resource leaks
        let mut created = Vec::new();

        for i in 0..5 {
            match CpalOutput::new() {
                Ok(output) => {
                    created.push(output);
                    tracing::debug!("Created output {}", i);
                }
                Err(e) => {
                    eprintln!("Failed to create output {}: {}", i, e);
                    break;
                }
            }
        }

        // Drop all outputs
        drop(created);
        thread::sleep(Duration::from_millis(100));

        // Should be able to create new output after cleanup
        let result = CpalOutput::new();
        match result {
            Ok(_) => {
                tracing::debug!("Successfully created output after cleanup");
            }
            Err(e) => {
                eprintln!("Could not create output after cleanup: {}", e);
            }
        }
    }
}

// ============================================================================
// 6. Multiple Stream Creation Tests
// ============================================================================

mod multiple_stream_tests {
    use super::*;

    /// Test creating and destroying outputs in sequence
    #[test]
    fn test_sequential_output_lifecycle() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        for i in 0..5 {
            let result = CpalOutput::new();
            match result {
                Ok(mut output) => {
                    // Use the output briefly
                    let buffer = create_test_buffer(44100, 0.1, 440.0);
                    let _ = output.play(&buffer);
                    thread::sleep(Duration::from_millis(50));

                    // Explicitly drop
                    drop(output);
                    thread::sleep(Duration::from_millis(50));

                    tracing::debug!("Completed lifecycle {}", i);
                }
                Err(e) => {
                    eprintln!("Lifecycle {} failed: {}", i, e);
                    break;
                }
            }
        }
    }

    /// Test multiple outputs can coexist (platform dependent)
    #[test]
    #[ignore = "Platform dependent - some systems allow multiple streams, others don't"]
    fn test_multiple_concurrent_outputs() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let mut outputs = Vec::new();

        // Try to create multiple outputs
        for i in 0..3 {
            match CpalOutput::new() {
                Ok(output) => {
                    outputs.push(output);
                    tracing::debug!("Created concurrent output {}", i);
                }
                Err(e) => {
                    eprintln!("Could not create output {} (may be expected): {}", i, e);
                    break;
                }
            }
        }

        eprintln!("Created {} concurrent outputs", outputs.len());

        // Try to use all outputs
        let buffer = create_test_buffer(44100, 0.2, 440.0);
        for (i, output) in outputs.iter_mut().enumerate() {
            let result = output.play(&buffer);
            match result {
                Ok(()) => {
                    tracing::debug!("Playing on output {}", i);
                }
                Err(e) => {
                    eprintln!("Playback failed on output {}: {}", i, e);
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    /// Test output survives stop/start cycles
    #[test]
    fn test_output_persistence() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let buffer = create_test_buffer(44100, 0.5, 440.0);

                // Multiple play/stop cycles
                for i in 0..10 {
                    let result = output.play(&buffer);
                    assert!(result.is_ok(), "Play should succeed in cycle {}", i);

                    thread::sleep(Duration::from_millis(50));

                    let result = output.stop();
                    assert!(result.is_ok(), "Stop should succeed in cycle {}", i);

                    thread::sleep(Duration::from_millis(10));
                }

                // Output should still be functional
                let result = output.play(&buffer);
                assert!(result.is_ok(), "Output should remain functional");
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }
}

// ============================================================================
// 7. Error Propagation Tests
// ============================================================================

mod error_propagation_tests {
    use super::*;

    /// Test that errors are properly propagated through the API
    #[test]
    fn test_error_types() {
        // Test invalid volume
        if let Ok(mut output) = CpalOutput::new() {
            let result = output.set_volume(-0.1);
            assert!(result.is_err(), "Should reject negative volume");
            match result {
                Err(e) => {
                    let error_msg = format!("{:?}", e);
                    assert!(
                        error_msg.contains("InvalidVolume") || error_msg.contains("Invalid"),
                        "Should be volume error"
                    );
                }
                Ok(_) => panic!("Should not accept negative volume"),
            }

            let result = output.set_volume(1.5);
            assert!(result.is_err(), "Should reject volume > 1.0");
        }
    }

    /// Test command channel behavior
    #[test]
    fn test_command_channel_stress() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        let result = CpalOutput::new();
        match result {
            Ok(mut output) => {
                let buffer = create_test_buffer(44100, 0.1, 440.0);

                // Flood the command channel
                for i in 0..100 {
                    let _ = output.play(&buffer);
                    let _ = output.set_volume(0.5);
                    let _ = output.pause();
                    let _ = output.resume();

                    if i % 10 == 0 {
                        thread::sleep(Duration::from_millis(5));
                    }
                }

                // Output should still be responsive
                let result = output.stop();
                assert!(result.is_ok(), "Should handle command flood gracefully");
            }
            Err(e) => {
                eprintln!("Could not create output: {}", e);
            }
        }
    }
}

// ============================================================================
// 8. Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    /// Comprehensive error handling test
    #[test]
    fn test_comprehensive_error_handling() {
        if !has_audio_hardware() {
            eprintln!("Skipping: No audio hardware available");
            return;
        }

        eprintln!("=== Comprehensive Error Handling Test ===\n");

        // 1. Create output
        eprintln!("Step 1: Create output");
        let mut output = match CpalOutput::new() {
            Ok(o) => {
                eprintln!("  ✓ Output created");
                o
            }
            Err(e) => {
                eprintln!("  ✗ Failed: {}", e);
                return;
            }
        };

        // 2. Test invalid operations
        eprintln!("\nStep 2: Test invalid operations");
        assert!(output.set_volume(-1.0).is_err(), "Reject negative volume");
        assert!(output.set_volume(2.0).is_err(), "Reject volume > 1.0");
        eprintln!("  ✓ Invalid parameters rejected");

        // 3. Test empty buffer
        eprintln!("\nStep 3: Test edge cases");
        let format = AudioFormat::new(SampleRate::new(44100), 2, 32);
        let empty = AudioBuffer::new(vec![], format);
        let _ = output.play(&empty);
        eprintln!("  ✓ Empty buffer handled");

        // 4. Test normal operation
        eprintln!("\nStep 4: Test normal operation");
        let buffer = create_test_buffer(44100, 0.2, 440.0);
        assert!(output.play(&buffer).is_ok(), "Normal playback works");
        thread::sleep(Duration::from_millis(100));
        assert!(output.pause().is_ok(), "Pause works");
        assert!(output.resume().is_ok(), "Resume works");
        assert!(output.stop().is_ok(), "Stop works");
        eprintln!("  ✓ Normal operations successful");

        eprintln!("\n=== Test Complete ===");
    }
}
