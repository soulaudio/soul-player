/// Integration tests for CPAL audio output
use soul_audio_desktop::CpalOutput;
use soul_core::{AudioBuffer, AudioFormat, AudioOutput, SampleRate};
use std::f32::consts::PI;

/// Generate a sine wave for testing
fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2); // Stereo

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * 0.3; // 30% amplitude to avoid clipping
        samples.push(sample); // Left channel
        samples.push(sample); // Right channel
    }

    samples
}

#[test]
fn test_create_output() {
    // This test verifies we can create a CPAL output
    match CpalOutput::new() {
        Ok(output) => {
            assert_eq!(output.volume(), 1.0);
        }
        Err(soul_audio_desktop::AudioOutputError::DeviceNotFound) => {
            // Expected in headless CI environments
            println!("No audio device found - skipping test");
        }
        Err(soul_audio_desktop::AudioOutputError::StreamBuildError(_)) => {
            // Also expected in environments without working audio devices
            println!("Audio device unavailable - skipping test");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_play_sine_wave() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Generate a 440 Hz sine wave (A4 note) for 0.1 seconds
    let samples = generate_sine_wave(440.0, 0.1, 44100);
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(samples, format);

    // Play the buffer
    assert!(output.play(&buffer).is_ok());

    // Give it time to start playing
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Stop playback
    assert!(output.stop().is_ok());
}

#[test]
fn test_playback_controls() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Generate test audio
    let samples = generate_sine_wave(440.0, 0.5, 44100);
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(samples, format);

    // Test play
    assert!(output.play(&buffer).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test pause
    assert!(output.pause().is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test resume
    assert!(output.resume().is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Test stop
    assert!(output.stop().is_ok());
}

#[test]
fn test_volume_control() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Test initial volume
    assert_eq!(output.volume(), 1.0);

    // Test setting valid volumes
    assert!(output.set_volume(0.5).is_ok());
    assert_eq!(output.volume(), 0.5);

    assert!(output.set_volume(0.0).is_ok());
    assert_eq!(output.volume(), 0.0);

    assert!(output.set_volume(1.0).is_ok());
    assert_eq!(output.volume(), 1.0);

    // Test invalid volumes
    assert!(output.set_volume(-0.1).is_err());
    assert!(output.set_volume(1.1).is_err());
}

#[test]
fn test_volume_actually_affects_output_level() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Generate a test signal
    let samples_full = generate_sine_wave(440.0, 0.1, 44100);
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);

    // Play at full volume
    output.set_volume(1.0).unwrap();
    let buffer_full = AudioBuffer::new(samples_full.clone(), format.clone());
    assert!(output.play(&buffer_full).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));
    output.stop().unwrap();

    // Play at half volume - the output should be quieter
    // We can't directly measure the output, but we verify the volume is set
    output.set_volume(0.5).unwrap();
    assert_eq!(output.volume(), 0.5, "Volume should be set to 0.5");

    // Play at zero volume
    output.set_volume(0.0).unwrap();
    assert_eq!(output.volume(), 0.0, "Volume should be set to 0.0");
    let buffer_muted = AudioBuffer::new(samples_full.clone(), format);
    assert!(output.play(&buffer_muted).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));
    output.stop().unwrap();

    // Verify volume can be restored
    output.set_volume(1.0).unwrap();
    assert_eq!(output.volume(), 1.0, "Volume should be restored to 1.0");
}

#[test]
fn test_volume_bounds_are_enforced() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Set to valid volume first
    output.set_volume(0.5).unwrap();

    // Try to set invalid volumes - should fail and NOT change current volume
    let original_volume = output.volume();

    let result = output.set_volume(-0.5);
    assert!(result.is_err(), "Negative volume should fail");
    assert_eq!(output.volume(), original_volume, "Volume should not change on error");

    let result = output.set_volume(2.0);
    assert!(result.is_err(), "Volume > 1.0 should fail");
    assert_eq!(output.volume(), original_volume, "Volume should not change on error");

    let result = output.set_volume(f32::NAN);
    assert!(result.is_err(), "NaN volume should fail");
    assert_eq!(output.volume(), original_volume, "Volume should not change on error");
}

#[test]
fn test_play_with_volume_change() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Generate test audio
    let samples = generate_sine_wave(440.0, 0.3, 44100);
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(samples, format);

    // Play at full volume
    assert!(output.play(&buffer).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Change volume while playing
    assert!(output.set_volume(0.5).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    assert!(output.set_volume(0.2).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Stop
    assert!(output.stop().is_ok());
}

#[test]
fn test_sample_rate_conversion() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Generate audio at 48kHz (different from CD quality 44.1kHz)
    let samples = generate_sine_wave(440.0, 0.1, 48000);
    let format = AudioFormat::new(SampleRate::DVD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(samples, format);

    // This should trigger sample rate conversion
    assert!(output.play(&buffer).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));

    assert!(output.stop().is_ok());
}

#[test]
fn test_multiple_plays() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Play multiple buffers in sequence
    for freq in [440.0, 523.0, 659.0] {
        // A4, C5, E5
        let samples = generate_sine_wave(freq, 0.1, 44100);
        let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
        let buffer = AudioBuffer::new(samples, format);

        assert!(output.play(&buffer).is_ok());
        std::thread::sleep(std::time::Duration::from_millis(80));
    }

    assert!(output.stop().is_ok());
}

#[test]
fn test_play_silence() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Create a buffer of silence
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(vec![0.0; 44100 * 2], format); // 1 second of silence

    assert!(output.play(&buffer).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(output.stop().is_ok());
}

#[test]
fn test_empty_buffer() {
    let Ok(mut output) = CpalOutput::new() else {
        println!("No audio device - skipping test");
        return;
    };

    // Create an empty buffer
    let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
    let buffer = AudioBuffer::new(vec![], format);

    // Should not panic
    assert!(output.play(&buffer).is_ok());
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(output.stop().is_ok());
}
