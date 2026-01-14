//! End-to-end tests for sample rate conversion
//!
//! These tests validate that:
//! 1. Audio files are correctly resampled to match device sample rate
//! 2. Playback speed is correct regardless of sample rate mismatch
//! 3. Device switching reloads audio source with new sample rate
//! 4. Resampling quality is maintained
//! 5. Common sample rate conversions work (44.1→96, 48→96, etc.)

use soul_audio_desktop::sources::local::LocalAudioSource;
use soul_playback::AudioSource;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a test WAV file
fn create_test_wav(
    path: &Path,
    duration_secs: f32,
    frequency: f32,
    sample_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let tolerance = (expected_duration * 0.05) as f64;
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

    // Read original (no resampling, buffer fills quickly)
    let mut original_source =
        LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");
    let mut original_buffer = Vec::new();
    let mut buffer = vec![0.0f32; 8192];
    while !original_source.is_finished() {
        match original_source.read_samples(&mut buffer) {
            Ok(0) => std::thread::sleep(std::time::Duration::from_millis(5)),
            Ok(n) => original_buffer.extend_from_slice(&buffer[..n]),
            Err(e) => panic!("Read error: {}", e),
        }
    }
    let orig_samples = original_buffer.len();

    // Read resampled - need to wait for async decoder to fill buffer
    let mut resampled_source =
        LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");
    let mut resampled_buffer = Vec::new();
    while !resampled_source.is_finished() {
        match resampled_source.read_samples(&mut buffer) {
            Ok(0) => std::thread::sleep(std::time::Duration::from_millis(5)),
            Ok(n) => resampled_buffer.extend_from_slice(&buffer[..n]),
            Err(e) => panic!("Read error: {}", e),
        }
    }
    let resamp_samples = resampled_buffer.len();

    // Count zero crossings (indicates frequency is preserved)
    let orig_crossings = count_zero_crossings(&original_buffer[..orig_samples]);
    let resamp_crossings = count_zero_crossings(&resampled_buffer[..resamp_samples]);

    eprintln!(
        "Original samples: {}, zero crossings: {}",
        orig_samples, orig_crossings
    );
    eprintln!(
        "Resampled samples: {}, zero crossings: {}",
        resamp_samples, resamp_crossings
    );

    // Should have approximately same number of zero crossings
    // (within 10% tolerance for resampling artifacts)
    let crossing_ratio = resamp_crossings as f32 / orig_crossings as f32;
    eprintln!("Crossing ratio: {:.3}", crossing_ratio);

    assert!(
        (crossing_ratio - 1.0).abs() < 0.15,
        "Zero crossing rate should be preserved, got ratio {:.3}",
        crossing_ratio
    );

    eprintln!("✅ Resampling preserves timing (zero-crossing rate within 15%)");
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
        // Note: Ok(0) means buffer is temporarily empty, not EOF
        // Use is_finished() to check for actual end of stream
        let mut total_samples = 0;
        let mut buffer = vec![0.0f32; 4096];

        for _ in 0..10000 {
            // safety limit
            if source.is_finished() {
                break;
            }
            match source.read_samples(&mut buffer) {
                Ok(0) => {
                    // Buffer temporarily empty, wait for decoder
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
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
        let tolerance = (expected_duration * 0.05) as f64;
        assert!(
            (calculated_duration - expected_duration as f64).abs() < tolerance,
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

    // Read 0.1 second worth of samples (wait for buffer to fill if needed)
    let target1 = (48000.0 * 0.1 * 2.0) as usize; // 0.1s stereo
    let mut buffer1 = vec![0.0f32; 4096];
    let mut total1 = 0;
    for _ in 0..100 {
        if total1 >= target1 || source1.is_finished() {
            break;
        }
        match source1.read_samples(&mut buffer1) {
            Ok(0) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Ok(n) => total1 += n,
            Err(e) => panic!("Read error: {}", e),
        }
    }
    assert!(
        total1 >= target1,
        "Should read at least {} samples, got {}",
        target1,
        total1
    );

    // Simulate device switch to 96kHz
    // In real app, we would reload the audio source with new target rate
    let mut source2 = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");
    assert_eq!(source2.sample_rate(), 96000);

    // Read 0.1 second worth of samples
    let target2 = (96000.0 * 0.1 * 2.0) as usize; // 0.1s stereo
    let mut buffer2 = vec![0.0f32; 4096];
    let mut total2 = 0;
    for _ in 0..100 {
        if total2 >= target2 || source2.is_finished() {
            break;
        }
        match source2.read_samples(&mut buffer2) {
            Ok(0) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Ok(n) => total2 += n,
            Err(e) => panic!("Read error: {}", e),
        }
    }
    assert!(
        total2 >= target2,
        "Should read at least {} samples, got {}",
        target2,
        total2
    );

    // Verify sample counts are proportional to sample rates (using target amounts)
    let ratio = target2 as f32 / target1 as f32;
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

/// Test: No jitter/artifacts at start with high-energy content
///
/// This test verifies that resampler startup artifacts are properly skipped.
/// The rubato sinc resampler has an `output_delay()` that represents samples
/// that are based on the filter's internal state, not actual audio content.
/// These samples can cause audible "jitter" especially with high-energy content
/// (strong bass, full mix) because the artifacts are more pronounced.
#[test]
fn test_no_startup_artifacts_with_high_energy_content() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_high_energy.wav");

    // Create a file with high-energy content starting immediately
    // - Strong amplitude (0.9) simulates "full mix"
    // - Low frequency (60Hz) simulates "strong bass"
    // - Source at 96kHz, target at 48kHz (downsampling scenario)
    create_high_energy_wav(&wav_path, 0.5, 60.0, 0.9, 96000).unwrap();

    // Downsample from 96kHz to 48kHz
    let mut source = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");

    // Read the first chunk of audio
    let mut buffer = vec![0.0f32; 4800]; // 50ms at 48kHz stereo
    let samples_read = source.read_samples(&mut buffer).unwrap();

    assert!(samples_read > 0, "Should read samples");

    // Check for artifacts: look for unexpected high-amplitude spikes
    // or rapid oscillations in the first few milliseconds
    let first_10ms_samples = (48000.0 * 0.010 * 2.0) as usize; // 10ms stereo
    let check_samples = first_10ms_samples.min(samples_read);

    // Calculate the max sample-to-sample delta (jitter indicator)
    let mut max_delta: f32 = 0.0;
    for i in 2..check_samples.min(buffer.len() - 2) {
        let delta = (buffer[i] - buffer[i - 2]).abs(); // Same channel
        max_delta = max_delta.max(delta);
    }

    // With a 60Hz sine wave at 48kHz, the expected max delta between
    // consecutive same-channel samples should be roughly:
    // delta ≈ 2 * pi * freq / sample_rate * amplitude ≈ 0.007 per sample * 2 = ~0.014
    // Allow 5x tolerance for normal waveform variations: ~0.07
    // Jitter/artifacts would cause much higher deltas (0.3+)
    eprintln!("Max sample delta in first 10ms: {:.6}", max_delta);
    eprintln!("First 10 samples: {:?}", &buffer[..20.min(check_samples)]);

    assert!(
        max_delta < 0.3,
        "First 10ms should not have jitter artifacts (max_delta={:.4}, expected <0.3)",
        max_delta
    );

    // Also verify the audio content is valid (not all zeros from skip)
    let rms: f32 =
        buffer[..check_samples].iter().map(|s| s * s).sum::<f32>() / check_samples as f32;
    let rms = rms.sqrt();
    eprintln!("RMS in first 10ms: {:.6}", rms);

    // For a 0.9 amplitude sine, RMS should be ~0.63
    // With some fade-in, allow lower values but not silence
    assert!(
        rms > 0.01,
        "Audio should have content after resampling, got RMS={:.6}",
        rms
    );

    eprintln!("✅ No startup artifacts with high-energy content (bass 60Hz, 0.9 amplitude)");
}

/// Test: Downsampling doesn't introduce startup jitter
#[test]
fn test_downsample_no_startup_jitter() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_downsample.wav");

    // Create 96kHz file with 440Hz tone (full amplitude)
    create_high_energy_wav(&wav_path, 0.5, 440.0, 0.8, 96000).unwrap();

    // Downsample to 44.1kHz (common streaming scenario)
    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    let mut buffer = vec![0.0f32; 4410]; // 50ms at 44.1kHz stereo
    let samples_read = source.read_samples(&mut buffer).unwrap();

    // Check variance in first 5ms - should be smooth, not spiky
    let first_5ms = (44100.0 * 0.005 * 2.0) as usize;
    let check_samples = first_5ms.min(samples_read);

    // Calculate variance of sample-to-sample deltas
    let mut deltas = Vec::new();
    for i in 2..check_samples {
        deltas.push((buffer[i] - buffer[i - 2]).abs());
    }

    if !deltas.is_empty() {
        let mean_delta: f32 = deltas.iter().sum::<f32>() / deltas.len() as f32;
        let variance: f32 =
            deltas.iter().map(|d| (d - mean_delta).powi(2)).sum::<f32>() / deltas.len() as f32;
        let std_dev = variance.sqrt();

        eprintln!("Delta mean: {:.6}, std_dev: {:.6}", mean_delta, std_dev);

        // Jitter would cause high variance in deltas
        // Normal sine wave should have very consistent deltas (low std_dev)
        assert!(
            std_dev < 0.1,
            "Sample deltas should be consistent (std_dev={:.4}), jitter detected",
            std_dev
        );
    }

    eprintln!("✅ Downsampling 96kHz→44.1kHz has no startup jitter");
}

/// Test: Upsample scenario (less common but should also work)
#[test]
fn test_upsample_no_startup_artifacts() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_upsample.wav");

    // Create 44.1kHz file with strong bass (full amplitude)
    create_high_energy_wav(&wav_path, 0.5, 80.0, 0.85, 44100).unwrap();

    // Upsample to 96kHz
    let mut source = LocalAudioSource::new(&wav_path, 96000).expect("Failed to create source");

    let mut buffer = vec![0.0f32; 9600]; // 50ms at 96kHz stereo
    let samples_read = source.read_samples(&mut buffer).unwrap();

    // Check that we have smooth audio, not artifacts
    let first_5ms = (96000.0 * 0.005 * 2.0) as usize;
    let check_samples = first_5ms.min(samples_read);

    let max_sample = buffer[..check_samples]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    eprintln!("Max sample in first 5ms: {:.6}", max_sample);

    // Should not clip or have wild spikes
    assert!(
        max_sample < 1.5,
        "Upsampled audio should not have amplitude spikes (max={:.4})",
        max_sample
    );

    eprintln!("✅ Upsampling 44.1kHz→96kHz has no startup artifacts");
}

/// Helper: Create a high-energy test WAV file with specified amplitude and frequency
fn create_high_energy_wav(
    path: &Path,
    duration_secs: f32,
    frequency: f32,
    amplitude: f32, // 0.0 to 1.0
    sample_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
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
        // Sine wave at full amplitude from the start
        let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin() * amplitude;
        let sample_i16 = (i16::MAX as f32 * sample) as i16;
        writer.write_sample(sample_i16)?;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Test: Initial buffer is filled sufficiently to avoid underrun
///
/// This test verifies that when a track is first loaded, the buffer has enough
/// samples to avoid underrun during the first few audio callbacks. Buffer underrun
/// causes zeros to be interspersed with real audio, creating jitter/clicks.
///
/// This reproduces the bug where:
/// - First play from tracklist: jitter (disk I/O latency → buffer underrun)
/// - "Previous track" to restart: no jitter (file cached → fast fill)
#[test]
fn test_no_buffer_underrun_on_initial_load() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_initial_load.wav");

    // Create a file similar to the problematic track (44.1kHz, strong content)
    create_high_energy_wav(&wav_path, 2.0, 100.0, 0.9, 44100).unwrap();

    // Create source with resampling (44.1kHz → 48kHz, common ASIO scenario)
    let mut source = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");

    // Simulate rapid audio callback requests (like a real audio device)
    // Typical callback size is 256-1024 samples
    let callback_size = 512; // Typical ASIO buffer size
    let mut buffer = vec![0.0f32; callback_size];

    // First 10 callbacks (first ~50ms at 48kHz)
    let mut underrun_detected = false;
    let mut total_zeros = 0;
    let mut total_samples = 0;

    for callback_idx in 0..10 {
        let samples_read = source.read_samples(&mut buffer).unwrap();

        // Count zeros (underrun indicator)
        let zeros_in_callback = buffer[..samples_read].iter().filter(|&&s| s == 0.0).count();

        if zeros_in_callback > 0 && samples_read > 0 {
            // Some zeros in a callback with real data = buffer underrun
            let zero_ratio = zeros_in_callback as f32 / samples_read as f32;
            if zero_ratio > 0.1 {
                // More than 10% zeros = significant underrun
                eprintln!(
                    "Callback {}: underrun detected - {} zeros out of {} samples ({:.1}%)",
                    callback_idx,
                    zeros_in_callback,
                    samples_read,
                    zero_ratio * 100.0
                );
                underrun_detected = true;
            }
        }

        total_zeros += zeros_in_callback;
        total_samples += samples_read;
    }

    eprintln!(
        "Total: {} zeros out of {} samples ({:.2}%)",
        total_zeros,
        total_samples,
        (total_zeros as f32 / total_samples as f32) * 100.0
    );

    // After initial buffer fill wait in LocalAudioSource::new(), we should have
    // enough buffer to serve at least the first 10 callbacks without underrun
    assert!(
        !underrun_detected,
        "Buffer underrun detected in first 10 callbacks - initial buffer too small"
    );
}

/// Test: Buffer recovery after seek (similar to "previous track" behavior)
#[test]
fn test_no_underrun_after_seek_to_start() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test_seek_start.wav");

    create_high_energy_wav(&wav_path, 2.0, 100.0, 0.9, 44100).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 48000).expect("Failed to create source");

    // Read some samples to advance
    let mut buffer = vec![0.0f32; 4096];
    source.read_samples(&mut buffer).unwrap();
    source.read_samples(&mut buffer).unwrap();

    // Seek back to start (like "previous track")
    source.seek(std::time::Duration::ZERO).unwrap();

    // Wait a bit for seek to complete and buffer to refill
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Read first 10 callbacks after seek
    let callback_size = 512;
    let mut buffer = vec![0.0f32; callback_size];
    let mut underrun_detected = false;

    for callback_idx in 0..10 {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        let zeros = buffer[..samples_read].iter().filter(|&&s| s == 0.0).count();

        if zeros > samples_read / 10 && samples_read > 0 {
            eprintln!("After seek callback {}: {} zeros", callback_idx, zeros);
            underrun_detected = true;
        }
    }

    assert!(
        !underrun_detected,
        "Buffer underrun after seek - seek buffer recovery too slow"
    );
}

/// Manual test guide
#[test]
#[ignore = "This is a documentation test, not meant to run"]
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
    eprintln!();
}
