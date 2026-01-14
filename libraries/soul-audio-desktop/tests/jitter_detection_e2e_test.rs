//! End-to-end Jitter Detection Tests
//!
//! These tests detect and verify fixes for audio jitter/stuttering at playback start.
//!
//! The issue: When playing a track with strong bass or full mix at the start,
//! there's audible jitter in the first few milliseconds. This is caused by:
//! 1. Buffer underrun during initial disk I/O
//! 2. Improper handling of partial buffer fills
//!
//! Key observation: Jitter occurs on first play (cold cache) but NOT when using
//! "previous track" to restart (warm cache), indicating the issue is I/O related.
//!
//! These tests play actual audio through the full pipeline and analyze the output
//! for jitter artifacts.

use soul_audio_desktop::LocalAudioSource;
use soul_playback::{AudioSource, PlaybackConfig, PlaybackManager};
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

// ============================================================================
// TEST AUDIO GENERATION
// ============================================================================

/// Generate a test WAV file that mimics tracks with strong bass at start
/// (like ATTENTION.flac - Joji)
///
/// The file starts with an immediate full-amplitude low-frequency content
/// to stress-test the playback system's ability to handle demanding starts.
fn generate_bass_heavy_test_wav(path: &PathBuf, duration_secs: f64) -> std::io::Result<()> {
    let sample_rate = 48000;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2;

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
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?; // 16-bit

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    // Generate audio that starts with immediate strong bass + mids
    // This mimics tracks like ATTENTION.flac that start with full mix
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;

        // Multiple frequencies for rich content
        let bass = (t * 60.0 * 2.0 * std::f64::consts::PI).sin() * 0.5; // 60Hz bass
        let sub_bass = (t * 40.0 * 2.0 * std::f64::consts::PI).sin() * 0.3; // 40Hz sub
        let mid = (t * 440.0 * 2.0 * std::f64::consts::PI).sin() * 0.2; // 440Hz mid
        let high = (t * 2000.0 * 2.0 * std::f64::consts::PI).sin() * 0.1; // 2kHz high

        // Immediate full amplitude (no fade-in in the source)
        let sample = bass + sub_bass + mid + high;

        // Soft clip to prevent harsh distortion
        let clipped = sample.tanh();
        let sample_i16 = (clipped * 30000.0) as i16;

        // Stereo (same on both channels)
        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

/// Generate a test WAV at a specific sample rate (for resampling tests)
fn generate_test_wav_at_rate(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f64,
) -> std::io::Result<()> {
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels: u32 = 2;

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels as usize * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&(sample_rate * channels * 2).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels as usize * 2) as u32).to_le_bytes())?;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let bass = (t * 60.0 * 2.0 * std::f64::consts::PI).sin() * 0.5;
        let sub_bass = (t * 40.0 * 2.0 * std::f64::consts::PI).sin() * 0.3;
        let mid = (t * 440.0 * 2.0 * std::f64::consts::PI).sin() * 0.2;
        let sample = (bass + sub_bass + mid).tanh();
        let sample_i16 = (sample * 30000.0) as i16;
        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

// ============================================================================
// JITTER ANALYSIS UTILITIES
// ============================================================================

/// Jitter detection result
#[derive(Debug)]
struct JitterAnalysis {
    /// Number of discontinuities detected (sample-to-sample jumps > threshold)
    discontinuities: usize,
    /// Number of silence gaps detected (consecutive near-zero samples)
    silence_gaps: usize,
    /// Maximum silence gap length in samples
    max_silence_gap: usize,
    /// Number of suspected buffer underruns
    underrun_events: usize,
    /// RMS level of first 10ms
    first_10ms_rms: f32,
    /// Maximum sample-to-sample jump in first 100ms
    max_jump_first_100ms: f32,
    /// Whether jitter was detected
    has_jitter: bool,
}

impl JitterAnalysis {
    fn new() -> Self {
        Self {
            discontinuities: 0,
            silence_gaps: 0,
            max_silence_gap: 0,
            underrun_events: 0,
            first_10ms_rms: 0.0,
            max_jump_first_100ms: 0.0,
            has_jitter: false,
        }
    }
}

/// Analyze audio buffer for jitter artifacts
///
/// Looks for:
/// 1. Large sample-to-sample discontinuities (indicates buffer underrun)
/// 2. Unexpected silence gaps (indicates missing samples)
/// 3. Repeated sample patterns (indicates buffer stalling)
///
/// NOTE: This analysis accounts for the normal fade-in behavior:
/// - First ~30ms has gradual amplitude increase (S-curve fade)
/// - DAC keepalive noise (~-96dB) before audio detected
/// - We skip the first 50ms when analyzing for silence gaps
fn analyze_for_jitter(samples: &[f32], sample_rate: u32) -> JitterAnalysis {
    let mut result = JitterAnalysis::new();

    if samples.len() < 4 {
        return result;
    }

    // Skip first 50ms for silence gap analysis (fade-in period)
    let fade_skip_samples = (sample_rate as usize * 2 / 20).min(samples.len()); // 50ms stereo

    // Analyze discontinuities (large sample-to-sample jumps)
    // Normal audio shouldn't have jumps > 0.3 between adjacent samples
    // We look for jumps AFTER the fade period to avoid flagging fade transitions
    let discontinuity_threshold = 0.4; // Slightly higher to account for bass content
    let mut max_jump = 0.0f32;

    let first_100ms_samples = (sample_rate as usize * 2 / 10).min(samples.len()); // stereo

    for i in (fade_skip_samples + 1)..samples.len() {
        let jump = (samples[i] - samples[i - 1]).abs();
        if jump > discontinuity_threshold {
            result.discontinuities += 1;
        }
    }

    // Track max jump in first 100ms (after fade)
    for i in (fade_skip_samples + 1)..first_100ms_samples {
        let jump = (samples[i] - samples[i - 1]).abs();
        if jump > max_jump {
            max_jump = jump;
        }
    }
    result.max_jump_first_100ms = max_jump;

    // Analyze silence gaps (AFTER fade period)
    // True silence gap = consecutive samples below noise floor with audio on both sides
    let silence_threshold = 0.0001; // Below -80dB is silence
    let mut current_gap = 0;

    let post_fade_samples = if samples.len() > fade_skip_samples {
        &samples[fade_skip_samples..]
    } else {
        samples
    };

    for &sample in post_fade_samples {
        if sample.abs() < silence_threshold {
            current_gap += 1;
        } else {
            // Only count gaps > 500 samples (~5ms) as suspicious
            // Smaller gaps could be zero crossings or normal quiet passages
            if current_gap > 500 {
                result.silence_gaps += 1;
                result.max_silence_gap = result.max_silence_gap.max(current_gap);
            }
            current_gap = 0;
        }
    }

    // Check final gap
    if current_gap > 500 {
        result.silence_gaps += 1;
        result.max_silence_gap = result.max_silence_gap.max(current_gap);
    }

    // Calculate RMS of first 10ms (will be low due to fade, that's expected)
    let first_10ms_samples = (sample_rate as usize * 2 / 100).min(samples.len()); // stereo
    if first_10ms_samples > 0 {
        let sum_sq: f32 = samples[..first_10ms_samples].iter().map(|s| s * s).sum();
        result.first_10ms_rms = (sum_sq / first_10ms_samples as f32).sqrt();
    }

    // Detect underrun events (patterns of: audio → hard silence → audio)
    // Only count transitions AFTER fade period and with substantial gaps
    let mut in_audio = false;
    let mut silence_count = 0;

    for &sample in post_fade_samples {
        if sample.abs() > 0.02 {
            // Higher threshold to avoid triggering on quiet passages
            if !in_audio && silence_count > 200 && silence_count < 10000 {
                // Substantial gap (>2ms) followed by audio = possible underrun
                result.underrun_events += 1;
            }
            in_audio = true;
            silence_count = 0;
        } else {
            if in_audio {
                silence_count = 0;
            }
            in_audio = false;
            silence_count += 1;
        }
    }

    // Determine if jitter is present
    // Jitter criteria (AFTER accounting for fade-in):
    // - Multiple discontinuities after fade period
    // - Long silence gaps (> 500 samples = ~5ms)
    // - Multiple underrun events
    // - Large sample jumps after fade
    result.has_jitter = result.discontinuities > 20
        || result.max_silence_gap > 1000  // ~10ms of silence is definitely bad
        || result.underrun_events > 2      // Allow 1-2 transitions (could be intentional)
        || result.max_jump_first_100ms > 0.6;

    result
}

/// Compare two audio buffers and detect differences
fn compare_audio_buffers(first: &[f32], second: &[f32]) -> f32 {
    let len = first.len().min(second.len());
    if len == 0 {
        return 0.0;
    }

    let mut diff_sum = 0.0f32;
    for i in 0..len {
        diff_sum += (first[i] - second[i]).abs();
    }

    diff_sum / len as f32
}

// ============================================================================
// END-TO-END JITTER TESTS
// ============================================================================

/// Test jitter detection on cold start (first play)
///
/// This simulates the user clicking play on a track for the first time.
/// The file is not in OS cache, so disk I/O is slow.
#[test]
fn test_jitter_detection_cold_start() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("bass_heavy_test.wav");

    // Generate 5 second test file with strong bass at start
    generate_bass_heavy_test_wav(&test_file, 5.0).expect("Failed to generate test file");

    eprintln!("[test] Created test file: {}", test_file.display());

    // Create audio source targeting 48kHz (common device rate)
    let source =
        LocalAudioSource::new(&test_file, 48000).expect("Failed to create LocalAudioSource");

    eprintln!("[test] Source created, duration: {:?}", source.duration());

    // Create PlaybackManager and play
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect first 500ms of audio output
    let samples_to_collect = 48000; // 500ms at 48kHz stereo
    let mut collected_samples = Vec::with_capacity(samples_to_collect);

    let mut buffer = vec![0.0f32; 2048];
    while collected_samples.len() < samples_to_collect {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        collected_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    eprintln!(
        "[test] Collected {} samples ({:.1}ms)",
        collected_samples.len(),
        collected_samples.len() as f32 / 96.0 // 48000 * 2 / 1000
    );

    // Analyze for jitter
    let analysis = analyze_for_jitter(&collected_samples, 48000);

    eprintln!("[test] Jitter Analysis:");
    eprintln!("  - Discontinuities: {}", analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", analysis.silence_gaps);
    eprintln!("  - Max silence gap: {} samples", analysis.max_silence_gap);
    eprintln!("  - Underrun events: {}", analysis.underrun_events);
    eprintln!("  - First 10ms RMS: {:.6}", analysis.first_10ms_rms);
    eprintln!(
        "  - Max jump (first 100ms): {:.4}",
        analysis.max_jump_first_100ms
    );
    eprintln!("  - Has jitter: {}", analysis.has_jitter);

    // The fade-in starts at very low amplitude, so RMS should be low initially
    // But we should NOT have jitter artifacts
    assert!(
        !analysis.has_jitter,
        "Jitter detected on cold start! Analysis: {:?}",
        analysis
    );

    assert!(
        analysis.max_silence_gap < 300,
        "Large silence gap detected: {} samples. This indicates buffer underrun.",
        analysis.max_silence_gap
    );
}

/// Test jitter detection on warm start (seek to beginning after load)
///
/// This simulates the user using "previous track" to restart - the file
/// is already in OS cache so disk I/O is fast.
#[test]
fn test_jitter_detection_warm_start() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("bass_heavy_test.wav");

    generate_bass_heavy_test_wav(&test_file, 5.0).expect("Failed to generate test file");

    eprintln!("[test] Created test file: {}", test_file.display());

    // Create and "warm up" the source by reading some data first
    let mut source =
        LocalAudioSource::new(&test_file, 48000).expect("Failed to create LocalAudioSource");

    // Read some data to warm the cache
    let mut warmup_buffer = vec![0.0f32; 48000]; // 500ms
    source.read_samples(&mut warmup_buffer).ok();

    // Seek back to start (simulates "previous track")
    source
        .seek(Duration::ZERO)
        .expect("Failed to seek to start");

    // Wait for buffer to refill after seek
    std::thread::sleep(Duration::from_millis(200));

    eprintln!("[test] Warmed up and seeked to start");

    // Create PlaybackManager with the warmed source
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect first 500ms
    let samples_to_collect = 48000;
    let mut collected_samples = Vec::with_capacity(samples_to_collect);

    let mut buffer = vec![0.0f32; 2048];
    while collected_samples.len() < samples_to_collect {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        collected_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    eprintln!("[test] Collected {} samples", collected_samples.len());

    let analysis = analyze_for_jitter(&collected_samples, 48000);

    eprintln!("[test] Warm Start Jitter Analysis:");
    eprintln!("  - Discontinuities: {}", analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", analysis.silence_gaps);
    eprintln!("  - Max silence gap: {}", analysis.max_silence_gap);
    eprintln!("  - Underrun events: {}", analysis.underrun_events);
    eprintln!("  - Has jitter: {}", analysis.has_jitter);

    assert!(
        !analysis.has_jitter,
        "Jitter detected on warm start! Analysis: {:?}",
        analysis
    );
}

/// Test that cold and warm starts produce similar output
///
/// If the playback system is working correctly, the audio output should
/// be nearly identical regardless of whether it's a cold or warm start.
#[test]
fn test_cold_vs_warm_start_consistency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("consistency_test.wav");

    generate_bass_heavy_test_wav(&test_file, 3.0).expect("Failed to generate test file");

    // Cold start: Create fresh source and collect output
    let source1 = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source 1");

    let mut manager1 = PlaybackManager::new(PlaybackConfig::default());
    manager1.set_sample_rate(48000);
    manager1.set_output_channels(2);
    manager1.set_audio_source(Box::new(source1));

    let mut cold_samples = Vec::with_capacity(24000);
    let mut buffer = vec![0.0f32; 2048];
    while cold_samples.len() < 24000 {
        let written = manager1.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        cold_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    // Warm start: Create source, read some, seek back
    let mut source2 = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source 2");

    // Warm up
    let mut warmup = vec![0.0f32; 48000];
    source2.read_samples(&mut warmup).ok();
    source2.seek(Duration::ZERO).expect("Failed to seek");
    std::thread::sleep(Duration::from_millis(200));

    let mut manager2 = PlaybackManager::new(PlaybackConfig::default());
    manager2.set_sample_rate(48000);
    manager2.set_output_channels(2);
    manager2.set_audio_source(Box::new(source2));

    let mut warm_samples = Vec::with_capacity(24000);
    while warm_samples.len() < 24000 {
        let written = manager2.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        warm_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    eprintln!(
        "[test] Cold samples: {}, Warm samples: {}",
        cold_samples.len(),
        warm_samples.len()
    );

    // Skip first ~50ms (fade-in period) and compare the rest
    // The fade should be consistent, but timing might differ slightly
    let skip_samples = 4800; // 50ms at 48kHz stereo

    if cold_samples.len() > skip_samples && warm_samples.len() > skip_samples {
        let cold_analysis = analyze_for_jitter(&cold_samples[skip_samples..], 48000);
        let warm_analysis = analyze_for_jitter(&warm_samples[skip_samples..], 48000);

        eprintln!("[test] Post-fade comparison:");
        eprintln!(
            "  - Cold discontinuities: {}, Warm: {}",
            cold_analysis.discontinuities, warm_analysis.discontinuities
        );
        eprintln!(
            "  - Cold silence gaps: {}, Warm: {}",
            cold_analysis.silence_gaps, warm_analysis.silence_gaps
        );

        // Both should have minimal jitter
        assert!(
            !cold_analysis.has_jitter,
            "Cold start has jitter after fade"
        );
        assert!(
            !warm_analysis.has_jitter,
            "Warm start has jitter after fade"
        );

        // The outputs should be similar (allowing for some variance in fade timing)
        let avg_diff =
            compare_audio_buffers(&cold_samples[skip_samples..], &warm_samples[skip_samples..]);

        eprintln!("[test] Average sample difference: {:.6}", avg_diff);

        assert!(
            avg_diff < 0.1,
            "Cold and warm outputs differ too much: {:.6}",
            avg_diff
        );
    }
}

/// Test jitter with resampling (44.1kHz source to 48kHz output)
///
/// Resampling adds additional complexity that could exacerbate jitter issues.
#[test]
fn test_jitter_with_resampling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("44100_test.wav");

    // Generate 44.1kHz source file
    generate_test_wav_at_rate(&test_file, 44100, 5.0).expect("Failed to generate test file");

    eprintln!("[test] Created 44.1kHz test file");

    // Create source with 48kHz target (will enable resampling)
    let source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

    eprintln!(
        "[test] Source needs resampling: {}",
        source.needs_resampling()
    );

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect first 500ms
    let mut collected = Vec::with_capacity(48000);
    let mut buffer = vec![0.0f32; 2048];

    while collected.len() < 48000 {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        collected.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    let analysis = analyze_for_jitter(&collected, 48000);

    eprintln!("[test] Resampling Jitter Analysis:");
    eprintln!("  - Discontinuities: {}", analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", analysis.silence_gaps);
    eprintln!("  - Max silence gap: {}", analysis.max_silence_gap);
    eprintln!("  - Underrun events: {}", analysis.underrun_events);
    eprintln!("  - Has jitter: {}", analysis.has_jitter);

    assert!(
        !analysis.has_jitter,
        "Jitter detected with resampling! Analysis: {:?}",
        analysis
    );
}

/// Test rapid seek operations don't cause jitter accumulation
#[test]
fn test_jitter_after_rapid_seeks() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("rapid_seek_test.wav");

    generate_bass_heavy_test_wav(&test_file, 10.0).expect("Failed to generate test file");

    let mut source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

    // Perform rapid seeks
    for i in 0..10 {
        let pos = Duration::from_secs(i % 5);
        source.seek(pos).ok();
        std::thread::sleep(Duration::from_millis(50));
    }

    // Seek to start
    source
        .seek(Duration::ZERO)
        .expect("Failed to seek to start");
    std::thread::sleep(Duration::from_millis(300)); // Let buffer refill

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect output
    let mut collected = Vec::with_capacity(48000);
    let mut buffer = vec![0.0f32; 2048];

    while collected.len() < 48000 {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        collected.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    let analysis = analyze_for_jitter(&collected, 48000);

    eprintln!("[test] Post-rapid-seek Jitter Analysis:");
    eprintln!("  - Discontinuities: {}", analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", analysis.silence_gaps);
    eprintln!("  - Max silence gap: {}", analysis.max_silence_gap);
    eprintln!("  - Has jitter: {}", analysis.has_jitter);

    assert!(
        !analysis.has_jitter,
        "Jitter detected after rapid seeks! Analysis: {:?}",
        analysis
    );
}

/// Test that the buffer is properly filled before playback starts
///
/// This directly tests the prebuffering fix - the source should have
/// significant samples ready before `process_audio` is first called.
#[test]
fn test_prebuffer_is_ready_before_playback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("prebuffer_test.wav");

    generate_bass_heavy_test_wav(&test_file, 3.0).expect("Failed to generate test file");

    let source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

    // Check that is_ready() returns true (buffer is pre-filled)
    assert!(
        source.is_ready(),
        "Source should be ready immediately after creation (prebuffered)"
    );

    eprintln!("[test] Source is_ready: {}", source.is_ready());

    // Even the first read should return substantial data
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    let mut buffer = vec![0.0f32; 4096];
    let first_read = manager.process_audio(&mut buffer).unwrap_or(0);

    eprintln!("[test] First read returned {} samples", first_read);

    // First read should return the full requested amount (prebuffer ensures this)
    assert!(
        first_read >= 4000,
        "First read should return most of requested samples, got {}",
        first_read
    );

    // Check that output isn't all zeros (which would indicate underrun)
    let non_zero_count = buffer[..first_read]
        .iter()
        .filter(|&&s| s.abs() > 1e-10)
        .count();
    let non_zero_ratio = non_zero_count as f32 / first_read as f32;

    eprintln!(
        "[test] Non-zero samples: {} ({:.1}%)",
        non_zero_count,
        non_zero_ratio * 100.0
    );

    // Most samples should be non-zero (allowing for fade starting at 0)
    // After fade starts, we should have audio content
    assert!(
        non_zero_ratio > 0.5,
        "Too many zero samples in first read: {:.1}% non-zero",
        non_zero_ratio * 100.0
    );
}

// ============================================================================
// REFERENCE COMPARISON TESTS (Compare with ffmpeg/external decoder)
// ============================================================================

/// Check if ffmpeg is available on the system
fn is_ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Decode audio file using ffmpeg and return raw PCM samples
///
/// This provides a reference decoder output to compare against Soul Player
fn decode_with_ffmpeg(input_path: &PathBuf, sample_rate: u32) -> Option<Vec<f32>> {
    if !is_ffmpeg_available() {
        eprintln!("[ffmpeg] ffmpeg not available, skipping reference decode");
        return None;
    }

    let temp_dir = TempDir::new().ok()?;
    let output_path = temp_dir.path().join("reference_output.raw");

    // Use ffmpeg to decode to raw 32-bit float PCM
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-i",
            input_path.to_str()?,
            "-f",
            "f32le", // 32-bit float little-endian
            "-acodec",
            "pcm_f32le",
            "-ar",
            &sample_rate.to_string(),
            "-ac",
            "2",  // stereo
            "-y", // overwrite
            output_path.to_str()?,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[ffmpeg] Decode failed: {}", stderr);
        return None;
    }

    // Read the raw PCM file
    let raw_data = std::fs::read(&output_path).ok()?;

    // Convert bytes to f32 samples
    let samples: Vec<f32> = raw_data
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    eprintln!(
        "[ffmpeg] Decoded {} samples ({:.2}s)",
        samples.len(),
        samples.len() as f32 / (sample_rate as f32 * 2.0)
    );

    Some(samples)
}

/// Cross-correlation to find alignment between two signals
fn find_alignment_offset(reference: &[f32], test: &[f32], max_offset: usize) -> (isize, f32) {
    let mut best_offset: isize = 0;
    let mut best_correlation: f32 = f32::MIN;

    let search_len = reference.len().min(test.len()).min(10000); // Limit search

    for offset in -(max_offset as isize)..=(max_offset as isize) {
        let mut correlation: f32 = 0.0;
        let mut count = 0;

        for i in 0..search_len {
            let ref_idx = i as isize;
            let test_idx = i as isize + offset;

            if ref_idx >= 0
                && (ref_idx as usize) < reference.len()
                && test_idx >= 0
                && (test_idx as usize) < test.len()
            {
                correlation += reference[ref_idx as usize] * test[test_idx as usize];
                count += 1;
            }
        }

        if count > 0 {
            correlation /= count as f32;
            if correlation > best_correlation {
                best_correlation = correlation;
                best_offset = offset;
            }
        }
    }

    (best_offset, best_correlation)
}

/// Calculate mean squared error between aligned signals
fn calculate_mse(reference: &[f32], test: &[f32], offset: isize) -> f32 {
    let mut sum_sq_error = 0.0f32;
    let mut count = 0;

    for i in 0..reference.len().min(test.len()) {
        let ref_idx = i as isize;
        let test_idx = i as isize + offset;

        if ref_idx >= 0
            && (ref_idx as usize) < reference.len()
            && test_idx >= 0
            && (test_idx as usize) < test.len()
        {
            let diff = reference[ref_idx as usize] - test[test_idx as usize];
            sum_sq_error += diff * diff;
            count += 1;
        }
    }

    if count > 0 {
        sum_sq_error / count as f32
    } else {
        f32::MAX
    }
}

/// Detect discontinuities by comparing derivative patterns
fn detect_discontinuity_differences(reference: &[f32], test: &[f32]) -> (usize, f32) {
    let len = reference.len().min(test.len());
    if len < 2 {
        return (0, 0.0);
    }

    let mut discontinuity_count = 0;
    let mut max_diff = 0.0f32;

    // Compare sample-to-sample differences (derivative)
    for i in 1..len {
        let ref_diff = (reference[i] - reference[i - 1]).abs();
        let test_diff = (test[i] - test[i - 1]).abs();

        let _derivative_error = (ref_diff - test_diff).abs();

        // If test has a much larger jump than reference, it's a potential discontinuity
        if test_diff > 0.2 && ref_diff < 0.1 {
            discontinuity_count += 1;
            max_diff = max_diff.max(test_diff);
        }
    }

    (discontinuity_count, max_diff)
}

/// Compare Soul Player output with ffmpeg reference decoder
///
/// This test decodes the same audio file using both Soul Player and ffmpeg,
/// then compares the outputs to detect any jitter or artifacts introduced
/// by Soul Player's pipeline.
#[test]
fn test_compare_with_ffmpeg_reference() {
    if !is_ffmpeg_available() {
        eprintln!("⊘ Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("reference_comparison.wav");

    // Generate test file
    generate_bass_heavy_test_wav(&test_file, 3.0).expect("Failed to generate test file");

    eprintln!("[test] Generated test file: {}", test_file.display());

    // Decode with ffmpeg (reference)
    let reference_samples =
        decode_with_ffmpeg(&test_file, 48000).expect("Failed to decode with ffmpeg");

    eprintln!(
        "[test] Reference samples: {} ({:.2}s)",
        reference_samples.len(),
        reference_samples.len() as f32 / 96000.0
    );

    // Decode with Soul Player
    let source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect Soul Player output
    let mut soul_samples = Vec::with_capacity(reference_samples.len());
    let mut buffer = vec![0.0f32; 4096];

    while soul_samples.len() < reference_samples.len() {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        soul_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    eprintln!(
        "[test] Soul Player samples: {} ({:.2}s)",
        soul_samples.len(),
        soul_samples.len() as f32 / 96000.0
    );

    // Skip the fade-in period (first 50ms) for comparison
    // Soul Player applies a fade-in that ffmpeg doesn't
    let skip_samples = 4800; // 50ms at 48kHz stereo

    if reference_samples.len() <= skip_samples || soul_samples.len() <= skip_samples {
        eprintln!("[test] Not enough samples for comparison");
        return;
    }

    let ref_post_fade = &reference_samples[skip_samples..];
    let soul_post_fade = &soul_samples[skip_samples..];

    // Find alignment (Soul Player may have slight timing offset due to buffering)
    let (offset, correlation) = find_alignment_offset(ref_post_fade, soul_post_fade, 1000);

    eprintln!(
        "[test] Alignment: offset={} samples, correlation={:.4}",
        offset, correlation
    );

    // Calculate MSE between aligned signals
    let mse = calculate_mse(ref_post_fade, soul_post_fade, offset);
    let rmse = mse.sqrt();

    eprintln!("[test] RMSE between signals: {:.6}", rmse);

    // Detect discontinuities that exist in Soul Player but not in reference
    let (disc_count, max_disc) = detect_discontinuity_differences(ref_post_fade, soul_post_fade);

    eprintln!(
        "[test] Discontinuity differences: count={}, max={:.4}",
        disc_count, max_disc
    );

    // Analyze both for jitter
    let ref_analysis = analyze_for_jitter(ref_post_fade, 48000);
    let soul_analysis = analyze_for_jitter(soul_post_fade, 48000);

    eprintln!("[test] Reference jitter analysis:");
    eprintln!("  - Discontinuities: {}", ref_analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", ref_analysis.silence_gaps);

    eprintln!("[test] Soul Player jitter analysis:");
    eprintln!("  - Discontinuities: {}", soul_analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", soul_analysis.silence_gaps);
    eprintln!("  - Max silence gap: {}", soul_analysis.max_silence_gap);

    // Soul Player should not have significantly MORE discontinuities than reference
    let extra_discontinuities = soul_analysis
        .discontinuities
        .saturating_sub(ref_analysis.discontinuities);

    assert!(
        extra_discontinuities < 50,
        "Soul Player has {} extra discontinuities compared to reference! \
         This indicates jitter in the playback pipeline.",
        extra_discontinuities
    );

    // Soul Player should not have silence gaps that reference doesn't have
    let extra_silence_gaps = soul_analysis
        .silence_gaps
        .saturating_sub(ref_analysis.silence_gaps);

    assert!(
        extra_silence_gaps == 0,
        "Soul Player has {} extra silence gaps compared to reference! \
         This indicates buffer underrun.",
        extra_silence_gaps
    );

    // RMSE should be reasonably low (allowing for volume processing differences)
    assert!(
        rmse < 0.3,
        "RMSE between Soul Player and reference is too high: {:.4}. \
         This indicates significant signal differences.",
        rmse
    );

    eprintln!("[test] ✓ Soul Player output matches reference decoder");
}

/// Test specific problematic scenario: strong bass transient at start
///
/// This specifically tests the case reported by the user where tracks
/// like ATTENTION.flac that start with strong bass cause jitter.
#[test]
fn test_strong_bass_transient_vs_reference() {
    if !is_ffmpeg_available() {
        eprintln!("⊘ Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("bass_transient.wav");

    // Generate file with VERY strong bass transient at the very start
    // This mimics tracks that start with immediate full-volume bass drop
    generate_extreme_bass_transient(&test_file).expect("Failed to generate test file");

    eprintln!("[test] Generated extreme bass transient file");

    // Get reference from ffmpeg
    let reference = decode_with_ffmpeg(&test_file, 48000).expect("Failed to decode reference");

    // Get Soul Player output
    let source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    let mut soul_samples = Vec::with_capacity(reference.len());
    let mut buffer = vec![0.0f32; 2048];

    while soul_samples.len() < reference.len() {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        soul_samples.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }

    // Analyze the critical first 100ms (where jitter is most audible)
    let first_100ms = 9600; // 100ms at 48kHz stereo

    let ref_first = &reference[..first_100ms.min(reference.len())];
    let soul_first = &soul_samples[..first_100ms.min(soul_samples.len())];

    let ref_analysis = analyze_for_jitter(ref_first, 48000);
    let soul_analysis = analyze_for_jitter(soul_first, 48000);

    eprintln!("[test] First 100ms analysis:");
    eprintln!(
        "  Reference: {} discontinuities, {} gaps",
        ref_analysis.discontinuities, ref_analysis.silence_gaps
    );
    eprintln!(
        "  Soul Player: {} discontinuities, {} gaps, max_gap={}",
        soul_analysis.discontinuities, soul_analysis.silence_gaps, soul_analysis.max_silence_gap
    );

    // Check for jitter-specific patterns
    let (disc_diff, max_disc) = detect_discontinuity_differences(ref_first, soul_first);

    eprintln!(
        "[test] Discontinuity differences in first 100ms: count={}, max={:.4}",
        disc_diff, max_disc
    );

    // Soul Player's output should be clean (fade handles transient smoothly)
    assert!(
        !soul_analysis.has_jitter,
        "Jitter detected in first 100ms of bass transient playback! \
         Analysis: {:?}",
        soul_analysis
    );

    // Should not have significant extra discontinuities vs reference
    assert!(
        disc_diff < 20,
        "Too many discontinuity differences ({}) in first 100ms. \
         This indicates jitter at playback start.",
        disc_diff
    );

    eprintln!("[test] ✓ Bass transient handled without jitter");
}

/// Generate a WAV file with extreme bass transient at the very start
fn generate_extreme_bass_transient(path: &PathBuf) -> std::io::Result<()> {
    let sample_rate = 48000;
    let duration_secs = 3.0;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2;

    let mut file = File::create(path)?;

    // WAV header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;

        // Extreme bass: 30Hz sub-bass at full amplitude from sample 0
        let sub_bass = (t * 30.0 * 2.0 * std::f64::consts::PI).sin() * 0.8;

        // 60Hz kick drum
        let kick = (t * 60.0 * 2.0 * std::f64::consts::PI).sin() * 0.6;

        // 808-style sub with harmonic
        let sub_808 = (t * 40.0 * 2.0 * std::f64::consts::PI).sin() * 0.5
            + (t * 80.0 * 2.0 * std::f64::consts::PI).sin() * 0.2;

        // Full mix from the start
        let sample = (sub_bass + kick + sub_808).tanh();
        let sample_i16 = (sample * 32000.0) as i16;

        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

/// Test with actual user-provided file path (if available)
///
/// Run with: `JITTER_TEST_FILE="D:\music\path\to\file.flac`" cargo test `test_with_user_file`
#[test]
fn test_with_user_file() {
    let test_file_path = std::env::var("JITTER_TEST_FILE").ok();

    if test_file_path.is_none() {
        eprintln!("⊘ Skipping test: Set JITTER_TEST_FILE env var to test with a specific file");
        eprintln!(
            "  Example: JITTER_TEST_FILE=\"D:\\music\\file.flac\" cargo test test_with_user_file"
        );
        return;
    }

    let path = PathBuf::from(test_file_path.unwrap());

    if !path.exists() {
        eprintln!("⊘ Test file does not exist: {}", path.display());
        return;
    }

    eprintln!("[test] Testing with user file: {}", path.display());

    // Create source
    let source = match LocalAudioSource::new(&path, 48000) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[test] Failed to create source: {}", e);
            return;
        }
    };

    eprintln!(
        "[test] Source created, duration: {:?}, needs_resampling: {}",
        source.duration(),
        source.needs_resampling()
    );

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(source));

    // Collect first 2 seconds
    let samples_to_collect = 48000 * 2 * 2; // 2 seconds stereo
    let mut collected = Vec::with_capacity(samples_to_collect);
    let mut buffer = vec![0.0f32; 4096];

    let start = Instant::now();
    while collected.len() < samples_to_collect {
        let written = manager.process_audio(&mut buffer).unwrap_or(0);
        if written == 0 {
            break;
        }
        collected.extend_from_slice(&buffer[..written.min(buffer.len())]);
    }
    let elapsed = start.elapsed();

    eprintln!(
        "[test] Collected {} samples in {:?}",
        collected.len(),
        elapsed
    );

    // Analyze for jitter
    let analysis = analyze_for_jitter(&collected, 48000);

    eprintln!("[test] User File Jitter Analysis:");
    eprintln!("  - Discontinuities: {}", analysis.discontinuities);
    eprintln!("  - Silence gaps: {}", analysis.silence_gaps);
    eprintln!("  - Max silence gap: {} samples", analysis.max_silence_gap);
    eprintln!("  - Underrun events: {}", analysis.underrun_events);
    eprintln!("  - First 10ms RMS: {:.6}", analysis.first_10ms_rms);
    eprintln!(
        "  - Max jump (first 100ms): {:.4}",
        analysis.max_jump_first_100ms
    );
    eprintln!("  - Has jitter: {}", analysis.has_jitter);

    // Compare with reference if ffmpeg available
    if is_ffmpeg_available() {
        if let Some(reference) = decode_with_ffmpeg(&path, 48000) {
            let skip = 4800; // Skip fade
            if reference.len() > skip && collected.len() > skip {
                let (disc_diff, max_disc) =
                    detect_discontinuity_differences(&reference[skip..], &collected[skip..]);
                eprintln!(
                    "[test] vs Reference: {} extra discontinuities, max={:.4}",
                    disc_diff, max_disc
                );
            }
        }
    }

    if analysis.has_jitter {
        eprintln!("[test] ⚠ JITTER DETECTED in user file!");
    } else {
        eprintln!("[test] ✓ No jitter detected in user file");
    }

    // Don't assert here - just report findings for user-provided files
}

// ============================================================================
// STRESS TESTS
// ============================================================================

/// Stress test: Multiple rapid play/seek cycles
#[test]
fn test_stress_rapid_play_seek_cycles() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("stress_test.wav");

    generate_bass_heavy_test_wav(&test_file, 10.0).expect("Failed to generate test file");

    let mut all_analyses = Vec::new();

    // Perform 10 play/seek cycles
    for cycle in 0..10 {
        let mut source = LocalAudioSource::new(&test_file, 48000).expect("Failed to create source");

        // Random seek
        let seek_pos = Duration::from_millis((cycle * 500) % 5000);
        source.seek(seek_pos).ok();
        std::thread::sleep(Duration::from_millis(100));

        // Seek back to start
        source.seek(Duration::ZERO).ok();
        std::thread::sleep(Duration::from_millis(100));

        let mut manager = PlaybackManager::new(PlaybackConfig::default());
        manager.set_sample_rate(48000);
        manager.set_output_channels(2);
        manager.set_audio_source(Box::new(source));

        // Collect output
        let mut collected = Vec::with_capacity(24000);
        let mut buffer = vec![0.0f32; 2048];

        while collected.len() < 24000 {
            let written = manager.process_audio(&mut buffer).unwrap_or(0);
            if written == 0 {
                break;
            }
            collected.extend_from_slice(&buffer[..written.min(buffer.len())]);
        }

        let analysis = analyze_for_jitter(&collected, 48000);
        all_analyses.push((cycle, analysis));
    }

    // Check results
    let mut jitter_cycles = 0;
    for (cycle, analysis) in &all_analyses {
        if analysis.has_jitter {
            jitter_cycles += 1;
            eprintln!(
                "[test] Cycle {} had jitter: discontinuities={}, gaps={}",
                cycle, analysis.discontinuities, analysis.silence_gaps
            );
        }
    }

    eprintln!("[test] Stress test: {}/10 cycles had jitter", jitter_cycles);

    assert!(
        jitter_cycles == 0,
        "{}/10 play/seek cycles had jitter! The pipeline is not stable.",
        jitter_cycles
    );

    eprintln!("[test] ✓ Stress test passed - no jitter in any cycle");
}
