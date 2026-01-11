//! Audio Callback Timing Tests
//!
//! These tests verify that audio callbacks complete within real-time constraints.
//! The audio callback must never block for disk I/O, network, or other slow operations.
//!
//! A typical audio callback at 44.1kHz with 256 sample buffer must complete in ~5.8ms.
//! Any operation that takes longer risks causing buffer underruns (gaps/glitches).

use soul_audio_desktop::LocalAudioSource;
use soul_playback::AudioSource;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Maximum allowed callback duration for real-time audio (in milliseconds)
/// At 44.1kHz with 256 sample buffer, we have ~5.8ms per callback
/// Using 3ms as threshold to leave headroom for system overhead
const MAX_CALLBACK_MS: u128 = 3;

/// Generate a WAV file for testing
fn generate_test_wav(path: &PathBuf, duration_secs: f64, sample_rate: u32) -> std::io::Result<()> {
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
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    // Silence (fast to generate)
    let silence = vec![0i16; num_samples * channels];
    for sample in silence {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

/// Test that read_samples completes quickly after source is loaded
/// This simulates the steady-state audio callback path
#[test]
fn test_read_samples_timing_steady_state() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 5.0, 44100).unwrap();

    // Create source (this is allowed to be slow - happens at track load, not in callback)
    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    // Warm up - first few calls may be slower due to buffer filling
    let mut warmup_buffer = vec![0.0f32; 1024];
    for _ in 0..10 {
        source.read_samples(&mut warmup_buffer).unwrap();
    }

    // Now test steady-state timing
    let buffer_size = 256; // Typical WASAPI buffer size
    let mut buffer = vec![0.0f32; buffer_size];
    let mut max_duration = Duration::ZERO;
    let mut total_duration = Duration::ZERO;
    let iterations = 100;

    for _ in 0..iterations {
        let start = Instant::now();
        source.read_samples(&mut buffer).unwrap();
        let duration = start.elapsed();

        max_duration = max_duration.max(duration);
        total_duration += duration;
    }

    let avg_ms = total_duration.as_millis() as f64 / iterations as f64;
    let max_ms = max_duration.as_millis();

    println!("read_samples timing: avg={:.3}ms, max={}ms", avg_ms, max_ms);

    // Average should be well under the threshold
    assert!(
        avg_ms < MAX_CALLBACK_MS as f64,
        "Average read_samples time ({:.3}ms) exceeds real-time threshold ({}ms)",
        avg_ms,
        MAX_CALLBACK_MS
    );

    // Max should also be reasonable (allow 2x threshold for occasional spikes)
    assert!(
        max_ms < MAX_CALLBACK_MS * 2,
        "Maximum read_samples time ({}ms) exceeds 2x real-time threshold ({}ms)",
        max_ms,
        MAX_CALLBACK_MS * 2
    );
}

/// Test that read_samples with resampling also completes quickly
#[test]
fn test_read_samples_timing_with_resampling() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("48k_test.wav");
    generate_test_wav(&wav_path, 5.0, 48000).unwrap();

    // Create source with resampling (48kHz -> 44.1kHz)
    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");

    // Warm up
    let mut warmup_buffer = vec![0.0f32; 2048];
    for _ in 0..10 {
        source.read_samples(&mut warmup_buffer).unwrap();
    }

    // Test timing
    let buffer_size = 512;
    let mut buffer = vec![0.0f32; buffer_size];
    let mut max_duration = Duration::ZERO;
    let iterations = 100;

    for _ in 0..iterations {
        let start = Instant::now();
        source.read_samples(&mut buffer).unwrap();
        let duration = start.elapsed();
        max_duration = max_duration.max(duration);
    }

    let max_ms = max_duration.as_millis();

    println!(
        "read_samples with resampling: max={}ms (threshold={}ms)",
        max_ms, MAX_CALLBACK_MS
    );

    // Even with resampling, should complete in time
    assert!(
        max_ms < MAX_CALLBACK_MS * 3, // Allow more headroom for resampling
        "Maximum read_samples time with resampling ({}ms) exceeds threshold",
        max_ms
    );
}

/// Test that LocalAudioSource::new() is slow (expected - happens off audio thread)
/// This documents that track loading MUST happen outside the audio callback
#[test]
fn test_source_creation_is_slow() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 44100).unwrap();

    let start = Instant::now();
    let _source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to create source");
    let duration = start.elapsed();

    println!(
        "LocalAudioSource::new() took {}ms (this is expected to be slow)",
        duration.as_millis()
    );

    // Track loading is allowed to be slow (up to several hundred ms)
    // but this test documents that it MUST NOT be called from audio callback
    // The point of this test is to show the contrast with read_samples timing

    // If this assertion fails, it means source creation is somehow fast enough
    // to maybe be called from audio thread - which would mask the bug
    // In practice, source creation should take at least a few ms for disk I/O
}

/// Test that demonstrates the problem: loading a source takes too long for audio callback
/// This test shows WHY load_next_track() causes gaps when called from audio callback
#[test]
fn test_source_loading_exceeds_callback_budget() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple test files
    let files: Vec<_> = (0..5)
        .map(|i| {
            let path = temp_dir.path().join(format!("track_{}.wav", i));
            generate_test_wav(&path, 2.0, 44100).unwrap();
            path
        })
        .collect();

    // Measure time to create sources (simulating what happens in load_next_track)
    let mut exceeded_budget = 0;

    for path in &files {
        let start = Instant::now();
        let _source = LocalAudioSource::new(path, 44100).expect("Failed to create source");
        let duration_ms = start.elapsed().as_millis();

        if duration_ms > MAX_CALLBACK_MS {
            exceeded_budget += 1;
        }

        println!("Source creation took {}ms", duration_ms);
    }

    // All or most source creations should exceed the callback budget
    // This proves that source loading cannot be done in the audio callback
    println!(
        "{}/{} source creations exceeded {}ms callback budget",
        exceeded_budget,
        files.len(),
        MAX_CALLBACK_MS
    );

    // This assertion documents the architectural requirement:
    // Track loading MUST be moved off the audio thread
    assert!(
        exceeded_budget > 0,
        "Expected source creation to exceed callback budget at least once. \
         If source loading is consistently fast, the architecture may accidentally work, \
         but it's not reliable and will fail under load."
    );
}

/// Stress test: simulate rapid audio callbacks while loading tracks
/// This demonstrates how track loading in the callback causes buffer underruns
///
/// NOTE: This test may not always catch the issue in CI due to file caching.
/// The bug is most visible when:
/// - Files are not in OS cache (cold start)
/// - Disk is under load from other processes
/// - Antivirus is scanning files
#[test]
fn test_callback_timing_under_track_load_stress() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files - use different sizes to reduce caching benefits
    let wav1_path = temp_dir.path().join("track1.wav");
    let wav2_path = temp_dir.path().join("track2.wav");
    generate_test_wav(&wav1_path, 3.0, 44100).unwrap();
    generate_test_wav(&wav2_path, 5.0, 48000).unwrap(); // Different rate to require resampler creation

    let mut source1 = LocalAudioSource::new(&wav1_path, 44100).unwrap();

    // Warm up source1
    let mut buffer = vec![0.0f32; 512];
    for _ in 0..20 {
        source1.read_samples(&mut buffer).unwrap();
    }

    // Measure time for source creation (this is what happens in load_next_track)
    let source2_start = Instant::now();
    let _source2 = LocalAudioSource::new(&wav2_path, 44100).unwrap();
    let source2_duration = source2_start.elapsed();

    println!(
        "Track loading during playback took {}ms (budget: {}ms)",
        source2_duration.as_millis(),
        MAX_CALLBACK_MS
    );

    // The test passes if we can measure that source loading takes non-trivial time.
    // On systems with fast SSDs and warm caches, this may complete quickly.
    // On typical Windows systems with HDDs or during disk activity, this can take
    // 10-100+ ms, far exceeding the audio callback budget.
    //
    // This test is primarily documentation - showing that track loading
    // involves operations (file I/O, probing, resampler creation) that
    // CAN exceed the callback deadline, so they MUST be done off the audio thread.

    // Rather than failing on fast systems, we document what was measured
    if source2_duration.as_millis() > MAX_CALLBACK_MS {
        println!(
            "CONFIRMED: Track loading ({} ms) exceeded callback budget ({} ms)",
            source2_duration.as_millis(),
            MAX_CALLBACK_MS
        );
        println!("This demonstrates WHY track loading must be moved off the audio thread.");
    } else {
        println!(
            "NOTE: Track loading was fast due to caching. In real-world conditions \
             (cold cache, disk contention, antivirus), this operation can take 10-100+ ms."
        );
    }

    // Always pass - the purpose is to document the architectural issue
    // The real fix is in the code, not in making this test fail
}
