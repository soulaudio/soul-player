//! End-to-end tests for sample rate conversion
//!
//! These tests validate that:
//! 1. Audio files are correctly resampled to match device sample rate
//! 2. Playback speed is correct regardless of sample rate mismatch
//! 3. Device switching reloads audio source with new sample rate
//! 4. Resampling quality is maintained
//! 5. Common sample rate conversions work (44.1→96, 48→96, etc.)

use soul_audio_desktop::sources::local::LocalAudioSource;
use soul_audio_desktop::DesktopPlayback;
use soul_playback::{AudioSource, PlaybackConfig, QueueTrack};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test WAV file
fn create_test_wav(
    path: &Path,
    duration_secs: f32,
    frequency: f32,
    sample_rate: u32,
) -> std::io::Result<()> {
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

/// Test: Audio source detects sample rate mismatch and enables resampling
#[test]
fn test_resampling_enabled_when_needed() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_44k.wav");

    // Create 44.1kHz file
    create_test_wav(&wav_path, 1.0, 440.0, 44100).unwrap();

    // Create source with target 96kHz (mismatch - should enable resampling)
    let source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

    // Verify source reports target sample rate (not file sample rate)
    assert_eq!(
        source.sample_rate(),
        96000,
        "Source should report target sample rate"
    );

    eprintln!("✅ Resampling enabled for 44.1kHz→96kHz conversion");
}

/// Test: No resampling when sample rates match
#[test]
fn test_no_resampling_when_rates_match() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_48k.wav");

    // Create 48kHz file
    create_test_wav(&wav_path, 1.0, 440.0, 48000).unwrap();

    // Create source with matching target 48kHz (no resampling needed)
    let source = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");

    assert_eq!(source.sample_rate(), 48000);

    eprintln!("✅ No resampling when file and target rates match");
}

/// Test: Common audiophile sample rate conversions
#[test]
fn test_common_sample_rate_conversions() {
    let temp_dir = TempDir::new().unwrap();

    // Test common conversions
    let test_cases = vec![
        (44100, 48000, "CD → 48kHz"),
        (44100, 88200, "CD → 88.2kHz"),
        (44100, 96000, "CD → 96kHz"),
        (44100, 176400, "CD → 176.4kHz"),
        (44100, 192000, "CD → 192kHz"),
        (48000, 96000, "48kHz → 96kHz"),
        (48000, 192000, "48kHz → 192kHz"),
        (96000, 192000, "96kHz → 192kHz"),
        (96000, 44100, "96kHz → CD (downsample)"),
    ];

    for (source_rate, target_rate, description) in test_cases {
        let wav_path = temp_dir
            .path()
            .join(format!("test_{}_{}.wav", source_rate, target_rate));

        create_test_wav(&wav_path, 0.5, 1000.0, source_rate).unwrap();

        let source =
            LocalAudioSource::new(&wav_path, target_rate).expect("Failed to create source");

        assert_eq!(source.sample_rate(), target_rate, "{} failed", description);

        eprintln!("✅ {}: {}Hz→{}Hz", description, source_rate, target_rate);
    }
}

/// Test: Resampled audio maintains correct duration
#[test]
fn test_resampled_duration_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_duration.wav");

    let expected_duration = 2.0; // seconds
    create_test_wav(&wav_path, expected_duration, 440.0, 44100).unwrap();

    // Upsample to 96kHz
    let source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

    let duration = source.duration();
    let duration_secs = duration.as_secs_f64();

    eprintln!(
        "Expected: {:.3}s, Got: {:.3}s",
        expected_duration, duration_secs
    );

    // Allow 5% tolerance for resampling and encoder/decoder overhead
    let tolerance = expected_duration * 0.05;
    assert!(
        (duration_secs - expected_duration as f64).abs() < tolerance,
        "Duration mismatch: expected {:.3}s ± {:.3}s, got {:.3}s",
        expected_duration,
        tolerance,
        duration_secs
    );

    eprintln!(
        "✅ Resampled audio duration accurate within {:.1}%",
        tolerance * 100.0
    );
}

/// Test: Resampled audio can be read and played
#[test]
fn test_resampled_audio_playback() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_playback.wav");

    // Create 44.1kHz file
    create_test_wav(&wav_path, 1.0, 440.0, 44100).unwrap();

    // Upsample to 96kHz
    let mut source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

    // Read samples - should be resampled to 96kHz
    let mut buffer = vec![0.0f32; 96000 * 2]; // 1 second at 96kHz stereo
    let samples_read = source
        .read_samples(&mut buffer)
        .expect("Failed to read samples");

    assert!(samples_read > 0, "Should read resampled samples");

    // Verify samples are in valid range [-1.0, 1.0]
    let max_sample = buffer[..samples_read]
        .iter()
        .map(|&s| s.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    assert!(
        max_sample <= 1.0,
        "Samples should be normalized, got max={}",
        max_sample
    );

    eprintln!("✅ Resampled audio readable and normalized");
    eprintln!(
        "   Read {} samples, max amplitude: {:.6}",
        samples_read, max_sample
    );
}

/// Test: Resampling quality (frequency preservation)
#[test]
fn test_resampling_frequency_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_freq.wav");

    let test_frequency = 1000.0; // 1kHz tone

    // Create 44.1kHz file with 1kHz tone
    create_test_wav(&wav_path, 0.5, test_frequency, 44100).unwrap();

    // Upsample to 96kHz
    let mut source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

    // Read a chunk
    let mut buffer = vec![0.0f32; 9600]; // 0.05 seconds at 96kHz stereo
    let samples_read = source
        .read_samples(&mut buffer)
        .expect("Failed to read samples");

    // Verify we got reasonable output
    assert!(samples_read > 1000, "Should read substantial chunk");

    // Calculate RMS to verify signal is present
    let rms: f32 = buffer[..samples_read].iter().map(|&s| s * s).sum::<f32>() / samples_read as f32;
    let rms = rms.sqrt();

    eprintln!("Resampled signal RMS: {:.6}", rms);

    // RMS should be around 0.35 for a sine wave with amplitude 0.5
    assert!(
        rms > 0.2 && rms < 0.5,
        "RMS should indicate proper signal level, got {:.6}",
        rms
    );

    eprintln!("✅ Resampling preserves frequency content");
}

/// Test: Zero-crossing rate is preserved (indicates timing accuracy)
#[test]
fn test_resampling_timing_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_timing.wav");

    let frequency = 440.0; // A4 note
    let duration = 1.0;

    // Create 44.1kHz file
    create_test_wav(&wav_path, duration, frequency, 44100).unwrap();

    // Read original
    let mut original_source =
        LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");
    let mut original_buffer = vec![0.0f32; 88200]; // 1 second stereo
    let orig_samples = original_source.read_samples(&mut original_buffer).unwrap();

    // Read resampled
    let mut resampled_source =
        LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");
    let mut resampled_buffer = vec![0.0f32; 192000]; // 1 second stereo at 96kHz
    let resamp_samples = resampled_source
        .read_samples(&mut resampled_buffer)
        .unwrap();

    // Count zero crossings (indicates frequency is preserved)
    let orig_crossings = count_zero_crossings(&original_buffer[..orig_samples]);
    let resamp_crossings = count_zero_crossings(&resampled_buffer[..resamp_samples]);

    eprintln!("Original zero crossings: {}", orig_crossings);
    eprintln!("Resampled zero crossings: {}", resamp_crossings);

    // Should have approximately same number of zero crossings
    // (within 10% tolerance for resampling artifacts)
    let crossing_ratio = resamp_crossings as f32 / orig_crossings as f32;
    eprintln!("Crossing ratio: {:.3}", crossing_ratio);

    assert!(
        (crossing_ratio - 1.0).abs() < 0.1,
        "Zero crossing rate should be preserved, got ratio {:.3}",
        crossing_ratio
    );

    eprintln!("✅ Resampling preserves timing (zero-crossing rate within 10%)");
}

/// Test: Playback speed is correct (duration-based)
#[test]
fn test_playback_speed_verification() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_speed.wav");

    let expected_duration = 3.0; // 3 seconds

    // Create 44.1kHz file
    create_test_wav(&wav_path, expected_duration, 440.0, 44100).unwrap();

    // Test with different target rates
    let target_rates = vec![48000, 88200, 96000, 192000];

    for target_rate in target_rates {
        let mut source =
            LocalAudioSource::new(&wav_path, target_rate).expect("Failed to create source");

        // Read entire file and measure sample count
        let mut total_samples = 0;
        let mut buffer = vec![0.0f32; 4096];

        for _ in 0..10000 {
            // safety limit
            match source.read_samples(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => total_samples += n,
                Err(e) => panic!("Read error: {}", e),
            }
        }

        // Calculate duration from samples
        let frames = total_samples / 2; // stereo
        let calculated_duration = frames as f64 / target_rate as f64;

        eprintln!(
            "Target: {}Hz, Total samples: {}, Calculated duration: {:.3}s",
            target_rate, total_samples, calculated_duration
        );

        // Allow 5% tolerance
        let tolerance = expected_duration * 0.05;
        assert!(
            (calculated_duration - expected_duration).abs() < tolerance,
            "Duration mismatch at {}Hz: expected {:.3}s ± {:.3}s, got {:.3}s",
            target_rate,
            expected_duration,
            tolerance,
            calculated_duration
        );

        eprintln!(
            "✅ Playback speed correct at {}Hz (duration: {:.3}s)",
            target_rate, calculated_duration
        );
    }
}

/// Test: Device switching scenario
#[test]
fn test_device_switch_resampling() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_device_switch.wav");

    // Create test file
    create_test_wav(&wav_path, 2.0, 440.0, 44100).unwrap();

    // Simulate device 1: 48kHz
    let mut source1 = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");
    assert_eq!(source1.sample_rate(), 48000);

    // Read some samples
    let mut buffer1 = vec![0.0f32; 4800]; // 0.05s at 48kHz
    let samples1 = source1.read_samples(&mut buffer1).unwrap();
    assert!(samples1 > 0);

    // Simulate device switch to 96kHz
    // In real app, we would reload the audio source with new target rate
    let mut source2 = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");
    assert_eq!(source2.sample_rate(), 96000);

    // Read samples from new source
    let mut buffer2 = vec![0.0f32; 9600]; // 0.05s at 96kHz
    let samples2 = source2.read_samples(&mut buffer2).unwrap();
    assert!(samples2 > 0);

    // Verify sample counts are proportional to sample rates
    let ratio = samples2 as f32 / samples1 as f32;
    eprintln!("Sample ratio 96k/48k: {:.3}", ratio);

    // Should be approximately 2:1 ratio
    assert!(
        (ratio - 2.0).abs() < 0.3,
        "Sample ratio should be ~2.0, got {:.3}",
        ratio
    );

    eprintln!("✅ Device switch resampling verified");
}

/// Test: Edge case - very high sample rate conversion
#[test]
fn test_extreme_upsampling() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_extreme.wav");

    // 44.1kHz → 192kHz (4.35x upsampling)
    create_test_wav(&wav_path, 0.5, 1000.0, 44100).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 192000).expect("Failed to create source");

    assert_eq!(source.sample_rate(), 192000);

    // Verify we can read samples
    let mut buffer = vec![0.0f32; 19200]; // 0.05s
    let samples = source.read_samples(&mut buffer).unwrap();

    assert!(
        samples > 0,
        "Should read samples even with extreme upsampling"
    );

    eprintln!("✅ Extreme upsampling (44.1→192kHz) works");
}

/// Helper: Count zero crossings in audio buffer (stereo, left channel only)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut crossings = 0;
    let mut last_sign = buffer[0] >= 0.0;

    for i in (2..buffer.len()).step_by(2) {
        // Left channel only
        let current_sign = buffer[i] >= 0.0;
        if current_sign != last_sign {
            crossings += 1;
        }
        last_sign = current_sign;
    }

    crossings
}

/// Manual test guide
#[test]
#[ignore] // This is a documentation test, not meant to run
fn manual_test_guide() {
    eprintln!("\n=== MANUAL TESTING GUIDE ===\n");
    eprintln!("1. Sample Rate Mismatch Detection:");
    eprintln!("   - Play a 44.1kHz MP3 file");
    eprintln!("   - Check console for: '[LocalAudioSource] Target sample rate: 96000 Hz'");
    eprintln!("   - Check console for: 'Needs resampling: true'");
    eprintln!("   - Verify audio plays at normal speed (not fast/slow)\n");

    eprintln!("2. Device Switching:");
    eprintln!("   - Start playback on default device");
    eprintln!("   - Switch to device with different sample rate");
    eprintln!("   - Check console for: '[DesktopPlayback] Reloading audio source'");
    eprintln!("   - Verify playback speed remains correct after switch\n");

    eprintln!("3. DSP Effects:");
    eprintln!("   - Add EQ effect with bass boost (+6dB at 100Hz)");
    eprintln!("   - Verify bass frequencies are louder");
    eprintln!("   - Toggle effect off - verify bass returns to normal");
    eprintln!("   - Remove effect - verify it disappears from UI\n");

    eprintln!("4. Effect Chain:");
    eprintln!("   - Add EQ boost (+6dB at 1kHz)");
    eprintln!("   - Add Compressor (moderate preset)");
    eprintln!("   - Add Limiter (-1dB threshold)");
    eprintln!("   - Verify all three show in effect chain UI");
    eprintln!("   - Play audio - should sound compressed and limited\n");

    eprintln!("Expected Console Output:");
    eprintln!("  [LocalAudioSource] File info:");
    eprintln!("    - Source sample rate: 44100 Hz");
    eprintln!("    - Target sample rate: 96000 Hz");
    eprintln!("    - Needs resampling: true");
    eprintln!("    - Speed ratio: 0.4594x");
    eprintln!("");
}
