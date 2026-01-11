//! Resampling Chunk Gap Detection Tests
//!
//! These tests verify that the LocalAudioSource produces continuous audio
//! when resampling is required. The bug occurs when:
//! 1. The resampler needs N frames (e.g., 1024) to produce output
//! 2. Decoded packets don't accumulate enough frames for a chunk
//! 3. read_samples() returns with output_buffer empty, filling silence
//!
//! This causes audible gaps/glitches during playback.

use soul_audio_desktop::LocalAudioSource;
use soul_playback::AudioSource;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Generate a WAV file at a specific sample rate
/// This allows testing resampling scenarios (source_rate != target_rate)
fn generate_wav_at_rate(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f64,
    frequency: f64,
) -> std::io::Result<()> {
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2; // Stereo

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?; // 16-bit

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    // Generate sine wave
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 32000.0) as i16; // Slightly below max to avoid clipping

        file.write_all(&sample_i16.to_le_bytes())?; // Left
        file.write_all(&sample_i16.to_le_bytes())?; // Right
    }

    Ok(())
}

/// Detect gaps (regions of near-silence) in audio data
/// Returns the count of gap regions found
fn detect_gaps(samples: &[f32], threshold: f32, min_gap_samples: usize) -> usize {
    let mut gap_count = 0;
    let mut consecutive_silent = 0;

    for sample in samples {
        if sample.abs() < threshold {
            consecutive_silent += 1;
        } else {
            if consecutive_silent >= min_gap_samples {
                gap_count += 1;
            }
            consecutive_silent = 0;
        }
    }

    gap_count
}

/// Calculate the RMS (root mean square) of a buffer
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

// ============================================================================
// RESAMPLING GAP DETECTION TESTS
// ============================================================================

/// Test that resampling from 48kHz to 44.1kHz produces continuous audio
/// This is the most common resampling scenario (many audio files are 48kHz)
#[test]
fn test_resampling_48k_to_44k_no_gaps() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("48k_test.wav");

    // Generate 48kHz source file
    generate_wav_at_rate(&wav_path, 48000, 2.0, 1000.0).unwrap();

    // Create source targeting 44.1kHz output (requires resampling)
    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    // Read audio in small chunks (256 samples = ~2.9ms at 44.1kHz stereo)
    // Small chunks are more likely to expose the gap bug
    let chunk_size = 256;
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; chunk_size];

    let mut zero_reads = 0;
    let mut partial_reads = 0;

    for _ in 0..1000 {
        // Enough iterations to read 2 seconds of audio
        let samples_read = source.read_samples(&mut buffer).unwrap();

        if samples_read == 0 {
            zero_reads += 1;
            if zero_reads > 5 {
                break; // True EOF
            }
            continue;
        }

        if samples_read < chunk_size {
            partial_reads += 1;
        }

        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Should have read substantial audio (close to 2 seconds)
    let expected_samples = 2.0 * 44100.0 * 2.0; // 2 sec * rate * stereo
    assert!(
        all_samples.len() as f64 > expected_samples * 0.9,
        "Should read close to 2 seconds of audio, got {} samples (expected ~{})",
        all_samples.len(),
        expected_samples
    );

    // Detect gaps (silence in the middle of audio)
    // Skip first and last 1000 samples (startup/shutdown transients)
    let inner_samples = if all_samples.len() > 2000 {
        &all_samples[1000..all_samples.len() - 1000]
    } else {
        &all_samples[..]
    };

    let gaps = detect_gaps(inner_samples, 0.001, 10);
    assert!(
        gaps == 0,
        "Found {} gap(s) in resampled audio - this indicates the chunk gap bug!",
        gaps
    );

    // Also verify overall audio level (not mostly silence)
    let audio_rms = rms(inner_samples);
    assert!(
        audio_rms > 0.1,
        "Audio RMS too low ({}), may indicate gap issue",
        audio_rms
    );

    println!(
        "Read {} samples, {} partial reads, RMS: {:.4}",
        all_samples.len(),
        partial_reads,
        audio_rms
    );
}

/// Test high ratio upsampling (22.05kHz to 44.1kHz)
#[test]
fn test_resampling_22k_to_44k_no_gaps() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("22k_test.wav");

    // Generate 22.05kHz source file (2x upsampling to 44.1kHz)
    generate_wav_at_rate(&wav_path, 22050, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    let chunk_size = 512;
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; chunk_size];

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Check for gaps
    let inner_samples = if all_samples.len() > 2000 {
        &all_samples[1000..all_samples.len() - 1000]
    } else {
        &all_samples[..]
    };

    let gaps = detect_gaps(inner_samples, 0.001, 10);
    assert!(gaps == 0, "Found {} gap(s) in upsampled audio!", gaps);
}

/// Test downsampling (96kHz to 44.1kHz)
#[test]
fn test_resampling_96k_to_44k_no_gaps() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("96k_test.wav");

    // Generate 96kHz source file
    generate_wav_at_rate(&wav_path, 96000, 1.0, 1000.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    let chunk_size = 1024;
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; chunk_size];

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    let inner_samples = if all_samples.len() > 2000 {
        &all_samples[1000..all_samples.len() - 1000]
    } else {
        &all_samples[..]
    };

    let gaps = detect_gaps(inner_samples, 0.001, 10);
    assert!(gaps == 0, "Found {} gap(s) in downsampled audio!", gaps);
}

/// Test with very small read chunks - most likely to trigger the bug
#[test]
fn test_resampling_with_tiny_chunks() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("tiny_chunk_test.wav");

    // Generate 48kHz source
    generate_wav_at_rate(&wav_path, 48000, 0.5, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    // Use very small chunks (64 samples = ~0.7ms)
    // This is much smaller than the resampler's internal chunk size (1024)
    // and is most likely to expose the gap bug
    let chunk_size = 64;
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; chunk_size];

    let mut consecutive_zeros = 0;
    let max_zero_reads = 10; // Should never have this many consecutive zero reads mid-file

    for i in 0..2000 {
        let samples_read = source.read_samples(&mut buffer).unwrap();

        if samples_read == 0 {
            consecutive_zeros += 1;
            if consecutive_zeros > max_zero_reads {
                // If we're getting many zero reads in a row, we're at EOF
                break;
            }
            // Zero read in the middle of the file is a bug!
            if !source.is_finished() && i < 1500 {
                // Allow zeros at end
                panic!(
                    "Got zero samples at iteration {} when not at EOF - chunk gap bug!",
                    i
                );
            }
            continue;
        }

        consecutive_zeros = 0;
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Verify we got reasonable output
    let expected_min_samples = 0.5 * 44100.0 * 2.0 * 0.8; // 80% of expected
    assert!(
        all_samples.len() as f64 > expected_min_samples,
        "Read too few samples: {} (expected > {})",
        all_samples.len(),
        expected_min_samples
    );
}

/// Test buffer boundary continuity - check for discontinuities at chunk boundaries
#[test]
fn test_resampling_chunk_boundary_continuity() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("boundary_test.wav");

    // Generate 48kHz sine wave
    generate_wav_at_rate(&wav_path, 48000, 1.0, 200.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    let chunk_size = 256;
    let mut buffer = vec![0.0f32; chunk_size];
    let mut prev_last_sample: Option<f32> = None;
    let mut large_jumps = 0;

    // For a 200Hz sine wave at 44.1kHz, max sample-to-sample change is:
    // 2*PI*200/44100 ≈ 0.0285 for unity amplitude
    // After resampling, amplitude may change, so use generous threshold
    let max_expected_jump = 0.5;

    for iteration in 0..100 {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }

        // Check for discontinuity at buffer boundary
        if let Some(prev) = prev_last_sample {
            let first = buffer[0];
            let jump = (first - prev).abs();
            if jump > max_expected_jump {
                large_jumps += 1;
                eprintln!(
                    "Large jump at iteration {}: prev={:.4}, first={:.4}, jump={:.4}",
                    iteration, prev, first, jump
                );
            }
        }

        // Store last sample (left channel)
        if samples_read >= 2 {
            prev_last_sample = Some(buffer[samples_read - 2]); // Left channel of last frame
        }
    }

    // Should have very few large jumps (maybe 1-2 at startup due to filter transients)
    assert!(
        large_jumps <= 2,
        "Too many discontinuities at buffer boundaries: {} (indicates gap bug)",
        large_jumps
    );
}

/// Stress test: rapid small reads during resampling
#[test]
fn test_resampling_rapid_small_reads() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("rapid_test.wav");

    generate_wav_at_rate(&wav_path, 48000, 3.0, 1000.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    // Simulate audio callback with varying small buffer sizes
    let buffer_sizes = [128, 256, 512, 128, 256, 128, 512, 256];
    let mut total_samples = 0;
    let mut zero_returns_mid_playback = 0;

    for _ in 0..500 {
        for &size in &buffer_sizes {
            let mut buffer = vec![0.0f32; size];
            let samples_read = source.read_samples(&mut buffer).unwrap();

            if samples_read == 0 {
                if !source.is_finished() {
                    zero_returns_mid_playback += 1;
                }
                break;
            }

            total_samples += samples_read;

            // Verify samples are finite
            for &s in &buffer[..samples_read] {
                assert!(s.is_finite(), "Got non-finite sample");
            }
        }

        if source.is_finished() {
            break;
        }
    }

    // Should not have any zero returns mid-playback
    assert!(
        zero_returns_mid_playback == 0,
        "Got {} zero returns during playback - indicates chunk gap bug!",
        zero_returns_mid_playback
    );

    // Should have read substantial audio
    let expected_samples = 3.0 * 44100.0 * 2.0;
    assert!(
        total_samples as f64 > expected_samples * 0.9,
        "Read too few samples: {} (expected ~{})",
        total_samples,
        expected_samples
    );
}

/// Test that no resampling case still works (regression check)
#[test]
fn test_no_resampling_no_gaps() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("same_rate.wav");

    // Source and target both 44.1kHz - no resampling needed
    generate_wav_at_rate(&wav_path, 44100, 1.0, 1000.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    let chunk_size = 256;
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; chunk_size];

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    let inner_samples = if all_samples.len() > 2000 {
        &all_samples[1000..all_samples.len() - 1000]
    } else {
        &all_samples[..]
    };

    let gaps = detect_gaps(inner_samples, 0.001, 10);
    assert!(gaps == 0, "Found {} gap(s) even without resampling!", gaps);

    let audio_rms = rms(inner_samples);
    assert!(audio_rms > 0.1, "Audio RMS too low: {}", audio_rms);
}
