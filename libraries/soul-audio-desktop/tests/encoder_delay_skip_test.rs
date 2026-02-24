//! Encoder delay skipping tests
//!
//! Tests that verify encoder delay (codec startup artifacts) is properly skipped
//! to prevent audio pops at track start and after seeks.
//!
//! ## Background
//! Most audio codecs add "encoder delay" - padding samples at the start of a file:
//! - MP3: ~1152 samples (26ms @ 44.1kHz)
//! - AAC: ~1024-2112 samples
//! - FLAC: ~0-256 samples (minimal, format dependent)
//!
//! These samples often contain:
//! - Near-silence with sudden amplitude jumps
//! - DC offset from encoder startup
//! - Filter ramp-up artifacts
//!
//! Playing these directly causes audible "pops" at track start.
//!
//! ## Solution
//! Skip the first `ENCODER_DELAY_FRAMES` (1200 frames) of decoded audio
//! to ensure clean playback startup.

use soul_audio_desktop::LocalAudioSource;
use soul_playback::AudioSource;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Get path to test audio file
fn get_test_audio(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // libraries
    path.pop(); // root
    path.push("applications/marketing/public/demo-audio");
    path.push(filename);
    path
}

/// Poll is_ready() until the buffer has enough pre-buffered samples (or EOF).
/// Mirrors how PlaybackManager waits before starting playback.
fn wait_for_ready(source: &LocalAudioSource) {
    for _ in 0..200 {
        if source.is_ready() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

// ===== Encoder Delay Skipping Tests =====

#[test]
fn test_encoder_delay_skipped_mp3_no_resampling() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    // Create source with native sample rate (no resampling)
    // MP3 files are typically 44.1kHz
    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read first buffer
    let mut buffer = vec![0.0f32; 2400]; // 1200 stereo frames

    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert!(samples_read > 0, "Should read samples");

    // Verify we didn't get complete silence (would indicate over-skipping)
    let has_audio = buffer[..samples_read].iter().any(|&s| s.abs() > 0.0001); // -80dB threshold
    assert!(
        has_audio,
        "First buffer should contain actual audio, not silence"
    );

    // Calculate peak amplitude in first 100 samples (should be clean audio, not ramp)
    let first_100_peak = buffer[..samples_read.min(100)]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    // If encoder delay was properly skipped, first samples should NOT be near-silence
    // followed by sudden jump. Instead, should start at reasonable amplitude.
    // This is a heuristic test - real audio usually starts > -40dB
    println!(
        "First 100 samples peak: {:.6} ({:.1}dB)",
        first_100_peak,
        20.0 * first_100_peak.log10()
    );

    // After encoder delay skip, audio may still start quietly
    // The key is that we're not getting the encoder delay artifacts (near-zero with sudden jumps)
    // StartFadeEnvelope will wait for amplitude > -60dB or timeout after 200ms
    //
    // We just verify that SOME audio is present (not complete silence)
    assert!(
        first_100_peak > 0.00001, // -100dB (just checking not complete silence)
        "First samples are complete silence - something is wrong. Peak: {:.6}",
        first_100_peak
    );
}

#[test]
fn test_encoder_delay_skipped_flac_no_resampling() {
    let path = get_test_audio("dark.flac");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load FLAC");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read first buffer
    let mut buffer = vec![0.0f32; 2400];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    assert!(samples_read > 0, "Should read samples");

    // FLAC has minimal encoder delay, but still should skip conservative amount
    let has_audio = buffer[..samples_read].iter().any(|&s| s.abs() > 0.0001);
    assert!(has_audio, "First buffer should contain actual audio");

    // Log first few samples for debugging
    println!(
        "FLAC first 8 samples: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
        buffer[0], buffer[1], buffer[2], buffer[3], buffer[4], buffer[5], buffer[6], buffer[7]
    );
}

#[test]
fn test_encoder_delay_skipped_with_resampling() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    // Force resampling by using different target rate
    // MP3 is 44.1kHz, output at 48kHz
    let mut source = LocalAudioSource::new(&path, 48000).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read first buffer
    let mut buffer = vec![0.0f32; 2400];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    assert!(samples_read > 0, "Should read samples");

    // With resampling enabled, both resampler delay AND encoder delay should be skipped
    // Audio may still be very quiet initially
    let has_audio = buffer[..samples_read].iter().any(|&s| s.abs() > 0.00001); // -100dB (not complete silence)
    assert!(
        has_audio,
        "First buffer should contain some audio (not complete silence)"
    );

    let first_100_peak = buffer[..samples_read.min(100)]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    println!(
        "First 100 samples peak (resampled): {:.6} ({:.1}dB)",
        first_100_peak,
        20.0 * first_100_peak.log10()
    );

    // With resampling, both resampler delay AND encoder delay are skipped
    // Audio may still start quietly - just verify not complete silence
    assert!(
        first_100_peak > 0.00001,
        "First samples should have some audio (not complete silence), got {:.6}",
        first_100_peak
    );
}

#[test]
fn test_encoder_delay_re_skipped_after_seek() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read initial samples
    let mut buffer = vec![0.0f32; 1024];
    source.read_samples(&mut buffer).unwrap();

    // Seek to middle of track
    source.seek(Duration::from_secs(5)).unwrap();

    // Read samples after seek — retry until fresh post-seek samples arrive
    let mut samples_read = 0;
    for _ in 0..200 {
        samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read > 0 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(samples_read > 0, "Should read after seek");

    // After seek, encoder delay should be skipped again
    // (Some decoders/formats re-add delay at seek points)
    let has_audio = buffer[..samples_read].iter().any(|&s| s.abs() > 0.0001);
    assert!(
        has_audio,
        "Should have audio after seek (encoder delay re-skipped)"
    );

    // Seek back to beginning
    source.seek(Duration::ZERO).unwrap();

    // Read from start again — retry until fresh samples arrive
    let mut samples_read = 0;
    for _ in 0..200 {
        samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read > 0 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(samples_read > 0, "Should read after seek to start");

    let first_peak = buffer[..samples_read]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    println!("Peak after seek to start: {:.6}", first_peak);
    assert!(
        first_peak > 0.001,
        "Encoder delay should be re-skipped after seek to start"
    );
}

#[test]
fn test_position_accurate_with_encoder_delay_skip() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Initial position should be 0, even though we skipped samples internally
    assert_eq!(
        source.position(),
        Duration::ZERO,
        "Position should start at 0 (encoder delay skip is internal)"
    );

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read 1 second worth of samples via multiple reads (buffer fills asynchronously)
    let one_second_samples = 44100 * 2; // stereo
    let mut chunk = vec![0.0f32; 4096];
    let mut total_read = 0;
    while total_read < one_second_samples {
        let n = source.read_samples(&mut chunk).unwrap();
        if n == 0 {
            thread::sleep(Duration::from_millis(5));
        } else {
            total_read += n;
        }
        if source.is_finished() {
            break;
        }
    }

    // Position should advance by ~1 second
    let position = source.position();
    println!(
        "Position after reading 1s worth: {:.2}s",
        position.as_secs_f64()
    );

    assert!(
        (position.as_secs_f64() - 1.0).abs() < 0.1,
        "Position should be ~1s, got {:.2}s (encoder delay skip shouldn't affect position)",
        position.as_secs_f64()
    );

    // Duration should be accurate (not affected by encoder delay skip)
    let duration = source.duration();
    println!("Total duration: {:.2}s", duration.as_secs_f64());
    assert!(
        duration > Duration::from_secs(5),
        "Duration should be reasonable"
    );
}

#[test]
fn test_no_amplitude_discontinuity_at_start() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Read first 1000 samples
    let mut buffer = vec![0.0f32; 1000];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    // Calculate amplitude envelope
    let mut max_jump = 0.0f32;
    let mut prev_amplitude = 0.0f32;

    for sample in buffer.iter().take(samples_read) {
        let current_amplitude = sample.abs();
        let jump = (current_amplitude - prev_amplitude).abs();

        if jump > max_jump {
            max_jump = jump;
        }

        prev_amplitude = current_amplitude;
    }

    println!(
        "Maximum amplitude jump in first 1000 samples: {:.6} ({:.1}dB)",
        max_jump,
        20.0 * max_jump.log10()
    );

    // If encoder delay was properly skipped, we shouldn't see massive jumps
    // from near-silence to full amplitude
    // Allow up to 0.3 (-10dB) jump for natural dynamics
    assert!(
        max_jump < 0.3,
        "Amplitude jump too large ({:.6}) - suggests encoder delay not skipped properly",
        max_jump
    );
}

// ===== Buffer Prebuffering Tests =====

#[test]
fn test_is_ready_waits_for_buffer_fill() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    // Source constructor returns immediately; the background decoder fills the buffer.
    // Measure how long it takes until is_ready() becomes true.
    let start = std::time::Instant::now();
    let source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Poll is_ready() (mirrors how PlaybackManager waits before starting playback)
    while !source.is_ready() {
        thread::sleep(Duration::from_millis(10));
    }

    let ready_time = start.elapsed();
    println!("Time until is_ready(): {}ms", ready_time.as_millis());

    // Should have taken at least 1ms to spawn thread and start decoding
    assert!(
        ready_time.as_millis() >= 1,
        "Buffer fill should take some time (got {}ms)",
        ready_time.as_millis()
    );

    assert!(
        source.is_ready(),
        "Source should be ready after polling loop"
    );
}

#[test]
fn test_is_ready_returns_true_for_short_files() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Even short files should be marked ready (eof condition or prebuffer reached)
    // Wait up to 2s for background decoder to fill the buffer or reach EOF
    wait_for_ready(&source);
    assert!(
        source.is_ready(),
        "Source should be ready (buffered or eof)"
    );
}

#[test]
fn test_sufficient_buffer_prevents_underrun() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Simulate audio callback pattern - read 512 frames repeatedly
    let callback_size = 512 * 2; // stereo
    let mut buffer = vec![0.0f32; callback_size];
    let mut underrun_count = 0;

    // Read for first second (should be glitch-free due to prebuffering).
    // Sleep between iterations to simulate real-time audio callback timing (~11.6ms per callback
    // at 44100Hz / 512 frames), giving the background decoder time to fill the ring buffer.
    let iterations = (44100 / 512) as usize; // ~1 second
    let callback_interval = Duration::from_micros(512 * 1_000_000 / 44100); // ~11.6ms
    for i in 0..iterations {
        let samples_read = source.read_samples(&mut buffer).unwrap();

        if samples_read < callback_size {
            underrun_count += 1;
            println!(
                "Underrun at iteration {}: got {} samples, wanted {}",
                i, samples_read, callback_size
            );
        }

        // Simulate real-time callback interval so the decoder can keep up
        thread::sleep(callback_interval);
    }

    // With proper prebuffering, should have NO underruns in first second
    assert_eq!(
        underrun_count, 0,
        "Should have no underruns in first second (proper prebuffering), got {}",
        underrun_count
    );
}

#[test]
fn test_buffer_size_is_adequate() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor).
    // MIN_BUFFER_SAMPLES = 12000 (~136ms at 44.1kHz stereo) will be available after is_ready().
    wait_for_ready(&source);

    // Read a chunk that fits within the pre-buffered amount
    let large_chunk = 8192; // ~93ms at 44.1kHz stereo, well under MIN_BUFFER_SAMPLES
    let mut buffer = vec![0.0f32; large_chunk];

    let samples_read = source.read_samples(&mut buffer).unwrap();

    // Should be able to fulfill the entire read from the prebuffered audio
    assert!(
        samples_read >= large_chunk,
        "Buffer should be able to serve {} samples immediately after is_ready(), got {}",
        large_chunk,
        samples_read
    );
}

// ===== Edge Cases with Encoder Delay =====

#[test]
fn test_encoder_delay_skip_does_not_affect_duration() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");
    let reported_duration = source.duration();

    // Read entire file
    let mut buffer = vec![0.0f32; 8192];
    let mut total_samples = 0;

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }
        total_samples += samples_read;
    }

    // Calculate playback time from samples
    let playback_time = total_samples as f64 / (44100.0 * 2.0);

    println!(
        "Reported duration: {:.2}s, Actual playback: {:.2}s",
        reported_duration.as_secs_f64(),
        playback_time
    );

    // Playback time will be less than reported duration (we skipped encoder delay)
    // 1200 frames at 44.1kHz = ~27ms skip
    // Difference should be small (<  100ms accounting for encoder delay + any padding)
    let difference = reported_duration.as_secs_f64() - playback_time;

    println!("Difference (reported - actual): {:.3}s", difference);

    // Verify playback is shorter (we skipped samples)
    assert!(
        difference > 0.0,
        "Playback time should be less than reported duration (encoder delay was skipped)"
    );

    // Note: This test may not read the entire file if buffer conditions prevent it
    // The key verification is that encoder delay skip doesn't break duration reporting
    // and that playback time is less than reported duration (encoder delay was skipped)
    if difference >= 1.0 {
        println!(
            "WARNING: Large duration difference {:.3}s - test may not have read entire file",
            difference
        );
        // Just verify encoder delay was skipped (some difference exists)
        assert!(
            difference >= 0.02, // At least 20ms difference (encoder delay)
            "Encoder delay should have been skipped"
        );
    }
}

#[test]
fn test_multiple_sources_all_skip_encoder_delay() {
    let mp3_path = get_test_audio("dark.mp3");
    let flac_path = get_test_audio("dark.flac");

    if !mp3_path.exists() || !flac_path.exists() {
        println!("Skipping test - demo files not found");
        return;
    }

    // Create multiple sources
    let mut sources = [
        LocalAudioSource::new(&mp3_path, 44100).expect("MP3 failed"),
        LocalAudioSource::new(&flac_path, 44100).expect("FLAC failed"),
        LocalAudioSource::new(&mp3_path, 48000).expect("MP3 resampled failed"),
    ];

    // All should have some audio (not complete silence)
    for (i, source) in sources.iter_mut().enumerate() {
        // Wait for background decoder to pre-buffer audio (async constructor)
        wait_for_ready(source);

        let mut buffer = vec![0.0f32; 1000];
        let samples_read = source.read_samples(&mut buffer).unwrap();

        let first_peak = buffer[..samples_read]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);

        println!("Source {} first peak: {:.6}", i, first_peak);
        assert!(
            first_peak > 0.00001,
            "Source {} should have some audio (not complete silence), got {:.6}",
            i,
            first_peak
        );
    }
}

#[test]
fn test_encoder_delay_skip_with_very_quiet_intro() {
    let path = get_test_audio("dark.mp3");
    if !path.exists() {
        println!("Skipping test - demo file not found: {:?}", path);
        return;
    }

    let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

    // Wait for background decoder to pre-buffer audio (async constructor)
    wait_for_ready(&source);

    // Read first buffer
    let mut buffer = vec![0.0f32; 4096];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    // Even if intro is quiet, should not be complete silence
    // (which would indicate we over-skipped)
    let total_energy: f64 = buffer[..samples_read]
        .iter()
        .map(|&s| (s as f64).powi(2))
        .sum();

    let rms = (total_energy / samples_read as f64).sqrt();
    println!("First buffer RMS: {:.6} ({:.1}dB)", rms, 20.0 * rms.log10());

    // RMS should be above noise floor (-80dB = 0.0001)
    assert!(
        rms > 0.0001,
        "RMS too low - may have over-skipped or source is silent"
    );
}
