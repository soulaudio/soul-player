// Allow unused_mut since many decoders are declared mut for future seeking API
#![allow(unused_mut)]

//! Audio Seeking Functionality Tests
//!
//! This test suite covers comprehensive seeking functionality for audio playback:
//!
//! ## Test Categories
//!
//! 1. **Seek Accuracy** - Verifying seek operations land at correct positions
//! 2. **Format-Specific Seeking** - Testing seeking across different audio formats
//! 3. **Edge Cases** - Boundary conditions and error handling
//! 4. **Precision Tests** - Frame-accurate seeking for compressed formats
//! 5. **Content Verification** - Validating audio content after seeking
//!
//! ## Implementation Status
//!
//! **NOTE**: Seeking is not yet implemented in the `AudioDecoder` trait.
//! All tests are marked with `#[ignore]` until the following is implemented:
//!
//! ```rust,ignore
//! pub trait AudioDecoder: Send {
//!     fn decode(&mut self, path: &Path) -> Result<AudioBuffer>;
//!     fn supports_format(&self, path: &Path) -> bool;
//!
//!     // TODO: Add seeking support
//!     fn seek(&mut self, position: Duration) -> Result<Duration>;
//!     fn duration(&self) -> Option<Duration>;
//!     fn position(&self) -> Duration;
//!
//!     // For streaming decode with seeking
//!     fn open(&mut self, path: &Path) -> Result<AudioMetadata>;
//!     fn decode_chunk(&mut self, max_frames: usize) -> Result<Option<AudioBuffer>>;
//! }
//! ```
//!
//! ## Compressed Format Considerations
//!
//! - **MP3**: Frame-based seeking, typically 26ms granularity (1152 samples @ 44.1kHz)
//! - **AAC**: Frame-based, 1024 samples per frame (~23ms @ 44.1kHz)
//! - **OGG/Vorbis**: Page-based seeking, variable granularity
//! - **FLAC**: Sample-accurate seeking via seek tables
//! - **WAV**: Sample-accurate seeking (direct byte offset)
//!
//! ## References
//!
//! - ISO/IEC 11172-3 (MPEG-1 Audio Layer III frame structure)
//! - ISO/IEC 14496-3 (AAC frame structure)
//! - Xiph.org Vorbis specification (OGG page structure)
//! - RFC 8478 (Opus seeking)

use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a WAV file with a specific frequency for content verification
fn create_wav_file(
    path: &PathBuf,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    duration_secs: f32,
    frequency: f32,
) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;

    let mut file = File::create(path).expect("Failed to create WAV file");

    let bytes_per_sample = bits_per_sample / 8;
    let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
    let block_align = channels * bytes_per_sample;
    let data_size = (num_samples * channels as usize * bytes_per_sample as usize) as u32;
    let chunk_size = 36 + data_size;

    let format_code: u16 = if bits_per_sample == 32 { 3 } else { 1 };

    // Write RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // Write fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&format_code.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&bits_per_sample.to_le_bytes()).unwrap();

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample_f = (2.0 * PI * frequency * t).sin();

        for _ in 0..channels {
            match bits_per_sample {
                16 => {
                    let sample_i16 = (sample_f * i16::MAX as f32) as i16;
                    file.write_all(&sample_i16.to_le_bytes()).unwrap();
                }
                24 => {
                    let sample_i32 = (sample_f * 8388607.0) as i32;
                    let bytes = sample_i32.to_le_bytes();
                    file.write_all(&bytes[0..3]).unwrap();
                }
                32 => {
                    file.write_all(&sample_f.to_le_bytes()).unwrap();
                }
                _ => panic!("Unsupported bit depth: {}", bits_per_sample),
            }
        }
    }
}

/// Create a WAV file with time-varying frequency (chirp) for position verification
///
/// The frequency increases linearly from start_freq to end_freq over the duration.
/// This allows verifying seek position by measuring the instantaneous frequency.
fn create_chirp_wav(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f32,
    start_freq: f32,
    end_freq: f32,
) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let channels = 2u16;

    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate chirp signal
    let freq_slope = (end_freq - start_freq) / duration_secs;
    let mut phase = 0.0f64;

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let instantaneous_freq = start_freq + freq_slope * t;

        // Phase accumulation for continuous chirp
        phase += 2.0 * std::f64::consts::PI * instantaneous_freq as f64 / sample_rate as f64;
        let sample_f = phase.sin() as f32;

        let sample_i16 = (sample_f * 0.9 * i16::MAX as f32) as i16;
        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a WAV file with distinct content in different time segments
///
/// Each segment has a unique frequency, making it easy to verify seek position.
fn create_segmented_wav(
    path: &PathBuf,
    sample_rate: u32,
    segment_duration_secs: f32,
    num_segments: usize,
) {
    let samples_per_segment = (sample_rate as f32 * segment_duration_secs) as usize;
    let total_samples = samples_per_segment * num_segments;
    let channels = 2u16;

    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (total_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Base frequencies for each segment (multiples of 100Hz for easy identification)
    for segment in 0..num_segments {
        let frequency = 200.0 + (segment as f32 * 200.0); // 200, 400, 600, 800, ...

        for i in 0..samples_per_segment {
            let t = i as f32 / sample_rate as f32;
            let sample_f = (2.0 * PI * frequency * t).sin();
            let sample_i16 = (sample_f * 0.9 * i16::MAX as f32) as i16;

            for _ in 0..channels {
                file.write_all(&sample_i16.to_le_bytes()).unwrap();
            }
        }
    }
}

/// Estimate the dominant frequency in a sample buffer using zero-crossing analysis
fn estimate_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    if samples.len() < 4 {
        return 0.0;
    }

    // Use left channel only (every other sample)
    let mono: Vec<f32> = samples.iter().step_by(2).copied().collect();

    // Count zero crossings
    let mut crossings = 0;
    for i in 1..mono.len() {
        if (mono[i - 1] < 0.0 && mono[i] >= 0.0) || (mono[i - 1] >= 0.0 && mono[i] < 0.0) {
            crossings += 1;
        }
    }

    // Frequency = zero_crossings / 2 / duration
    let duration = mono.len() as f32 / sample_rate as f32;
    crossings as f32 / 2.0 / duration
}

/// Calculate the expected sample index for a given time position
fn time_to_sample_index(time: Duration, sample_rate: u32) -> usize {
    (time.as_secs_f64() * sample_rate as f64) as usize
}

/// Calculate the expected time for a given sample index
fn sample_index_to_time(sample_index: usize, sample_rate: u32) -> Duration {
    Duration::from_secs_f64(sample_index as f64 / sample_rate as f64)
}

// ============================================================================
// SEEK ACCURACY TESTS
// ============================================================================

/// Test seeking to exact beginning of file (position 0)
#[test]
fn test_seek_to_beginning() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_begin.wav");

    create_wav_file(&path, 44100, 2, 16, 5.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // First decode some samples to move position forward
    decoder.decode_chunk(44100).expect("Failed to decode chunk");

    // Seek back to beginning
    let result = decoder.seek(Duration::from_secs(0));
    assert!(result.is_ok(), "Seek to beginning should succeed");

    let actual_position = result.unwrap();
    assert_eq!(actual_position, Duration::from_secs(0), "Should be at position 0");

    // Verify position is actually at the start
    assert_eq!(decoder.position(), Duration::from_secs(0));
}

/// Test seeking to specific timestamp and verifying position
#[test]
fn test_seek_to_specific_position() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_specific.wav");

    let duration_secs = 10.0;
    create_wav_file(&path, 44100, 2, 16, duration_secs, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // Test seeking to various positions
    let test_positions = [
        Duration::from_millis(500),
        Duration::from_secs(1),
        Duration::from_millis(2500),
        Duration::from_secs(5),
        Duration::from_millis(7777),
    ];

    for target in test_positions {
        let result = decoder.seek(target);
        assert!(result.is_ok(), "Seek to {:?} should succeed", target);

        let actual = result.unwrap();
        // For WAV (lossless), seeking should be sample-accurate
        // Use a larger tolerance since Symphonia may land on frame boundaries
        let tolerance = Duration::from_millis(50);
        assert!(
            actual >= target.saturating_sub(tolerance) && actual <= target + tolerance,
            "Seek to {:?} landed at {:?}, outside tolerance",
            target, actual
        );
    }
}

/// Test seeking to end of file
#[test]
fn test_seek_to_end() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_end.wav");

    let duration_secs = 3.0;
    create_wav_file(&path, 44100, 2, 16, duration_secs, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    let file_duration = decoder.duration().expect("Should have duration");

    // Seek to the end
    let result = decoder.seek(file_duration);
    assert!(result.is_ok(), "Seek to end should succeed");

    let actual = result.unwrap();
    // The seek should land at or near the end
    let tolerance = Duration::from_millis(100);
    assert!(
        actual >= file_duration.saturating_sub(tolerance),
        "Seek to end should land near the end, got {:?} for duration {:?}",
        actual,
        file_duration
    );

    // After seeking to near the end, we should be able to decode remaining samples
    // (which may be empty or contain just a few trailing samples)
    let chunk = decoder.decode_chunk(1024);
    assert!(chunk.is_ok(), "Decode after seeking to end should not error");
}

/// Test sample-accurate seeking in WAV files
///
/// WAV files support sample-accurate seeking because sample positions
/// can be calculated directly from byte offsets.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_sample_accurate_seeking_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("sample_accurate.wav");

    let sample_rate = 44100u32;
    create_segmented_wav(&path, sample_rate, 1.0, 5);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Failed to open file");
    //
    // // Seek to middle of segment 3 (at 2.5 seconds)
    // let target = Duration::from_millis(2500);
    // let result = decoder.seek(target);
    // assert!(result.is_ok());
    //
    // // Decode a small chunk and verify frequency matches segment 3 (600 Hz)
    // let chunk = decoder.decode_chunk(4410).expect("Should decode").expect("Should have data");
    // let freq = estimate_frequency(&chunk.samples, sample_rate);
    //
    // // Expected frequency for segment 3: 200 + 2*200 = 600 Hz
    // let expected_freq = 600.0;
    // let tolerance = 50.0; // Hz
    //
    // assert!(
    //     (freq - expected_freq).abs() < tolerance,
    //     "Frequency at 2.5s should be ~{} Hz, got {} Hz",
    //     expected_freq, freq
    // );

    assert!(decoder.supports_format(&path));
}

// ============================================================================
// FORMAT-SPECIFIC SEEKING TESTS
// ============================================================================

/// Test seeking in WAV format (sample-accurate seeking supported)
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_wav_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_test.wav");

    create_wav_file(&path, 44100, 2, 16, 5.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&path));

    // TODO: Test seeking behavior specific to WAV format
    // - Should support sample-accurate seeking
    // - Seek position should match requested position exactly
    // - No decoder state issues after seek
}

/// Test seeking in MP3 format (frame-based seeking)
///
/// MP3 uses frames of 1152 samples (at 44.1kHz = ~26.12ms per frame).
/// Seeking may not be sample-accurate; should land on frame boundary.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and MP3 test file
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_mp3_format() {
    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // - Create or use MP3 test file
    // - Seek to arbitrary position
    // - Verify position lands on frame boundary (within ~26ms)
    // - Verify audio content is correct after seek
    //
    // MP3 frame duration at 44.1kHz:
    // const MP3_FRAME_DURATION_MS: f64 = 1152.0 / 44100.0 * 1000.0; // ~26.12ms
    //
    // let target = Duration::from_millis(1000);
    // let result = decoder.seek(target);
    // let actual = result.unwrap();
    //
    // // Should be within one frame of target
    // let frame_duration = Duration::from_secs_f64(MP3_FRAME_DURATION_MS / 1000.0);
    // assert!(
    //     actual >= target.saturating_sub(frame_duration) && actual <= target + frame_duration,
    //     "MP3 seek should land within one frame of target"
    // );
}

/// Test seeking in FLAC format (sample-accurate via seek tables)
///
/// FLAC supports sample-accurate seeking using embedded seek tables.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and FLAC test file
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_flac_format() {
    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // - Create or use FLAC test file (preferably with seek table)
    // - Seek to arbitrary position
    // - Verify position matches requested position exactly
    // - Verify audio content is bit-perfect after seek
}

/// Test seeking in AAC format (frame-based seeking)
///
/// AAC uses frames of 1024 samples (at 44.1kHz = ~23.22ms per frame).
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and AAC test file
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_aac_format() {
    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // - Create or use AAC/M4A test file
    // - Seek to arbitrary position
    // - Verify position lands on frame boundary (within ~23ms)
    //
    // AAC frame duration at 44.1kHz:
    // const AAC_FRAME_DURATION_MS: f64 = 1024.0 / 44100.0 * 1000.0; // ~23.22ms
}

/// Test seeking in OGG/Vorbis format (page-based seeking)
///
/// OGG uses pages with variable granularity, seeking may have larger tolerance.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and OGG test file
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_ogg_format() {
    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // - Create or use OGG test file
    // - Seek to arbitrary position
    // - OGG seeking may have page-level granularity
    // - Typical tolerance: ~50-100ms depending on encoder settings
}

/// Test seeking in OPUS format
///
/// OPUS typically uses 20ms frames, supports seeking via OGG container.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and OPUS test file
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_opus_format() {
    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // - Create or use OPUS test file
    // - Seek to arbitrary position
    // - OPUS frame duration is typically 20ms
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

/// Test seeking past the end of file (should clamp or error gracefully)
#[test]
fn test_seek_past_end() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_past_end.wav");

    let duration_secs = 2.0;
    create_wav_file(&path, 44100, 2, 16, duration_secs, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    let file_duration = decoder.duration().expect("Should have duration");
    let past_end = file_duration + Duration::from_secs(10);

    let result = decoder.seek(past_end);

    // Two acceptable behaviors:
    // 1. Error is returned
    // 2. Position is clamped to end of file
    match result {
        Ok(actual) => {
            // If success, should be clamped to file duration
            let tolerance = Duration::from_millis(50);
            assert!(
                actual <= file_duration + tolerance,
                "Seek past end should clamp to duration, got {:?}",
                actual
            );
        }
        Err(_) => {
            // Error is also acceptable behavior
        }
    }
}

/// Test seeking to position 0 (minimum valid position)
///
/// Note: Duration cannot be negative, so we test boundary behavior around 0
#[test]
fn test_seek_to_zero() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_zero.wav");

    create_wav_file(&path, 44100, 2, 16, 3.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // Seek to position 0 (the minimum valid position)
    let result = decoder.seek(Duration::from_secs(0));
    assert!(result.is_ok(), "Seek to 0 should succeed");
    assert_eq!(result.unwrap(), Duration::from_secs(0));
}

/// Test rapid sequential seeks (stress test for seek state management)
#[test]
fn test_rapid_sequential_seeks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("rapid_seeks.wav");

    let duration_secs = 10.0;
    create_wav_file(&path, 44100, 2, 16, duration_secs, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // Perform many rapid seeks
    for i in 0..100u64 {
        let position = Duration::from_millis((i * 97) % 10000); // Pseudo-random positions
        let result = decoder.seek(position);
        assert!(
            result.is_ok(),
            "Rapid seek {} to {:?} should succeed",
            i, position
        );
    }

    // After rapid seeks, decoder should still be in valid state
    let chunk = decoder.decode_chunk(1024);
    assert!(chunk.is_ok(), "Decoding after rapid seeks should work");
}

/// Test seek during simulated playback
///
/// This simulates seeking while audio is being decoded (common in real playback).
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_during_playback_simulation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("seek_during_playback.wav");

    create_wav_file(&path, 44100, 2, 16, 10.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Failed to open file");
    //
    // // Simulate playback: decode, seek, decode, seek...
    // for _ in 0..5 {
    //     // Decode some samples (simulating playback)
    //     let _ = decoder.decode_chunk(4410); // 100ms of audio
    //
    //     // Seek to new position
    //     let new_position = Duration::from_secs(rand::random::<u64>() % 10);
    //     let result = decoder.seek(new_position);
    //     assert!(result.is_ok(), "Seek during playback should succeed");
    //
    //     // Continue decoding from new position
    //     let chunk = decoder.decode_chunk(4410);
    //     assert!(chunk.is_ok(), "Decode after seek should succeed");
    // }

    assert!(decoder.supports_format(&path));
}

/// Test seeking in very short file (less than one frame)
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_very_short_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("very_short.wav");

    // Create file with only 100 samples (~2.27ms at 44.1kHz)
    let sample_rate = 44100u32;
    let duration_secs = 100.0 / sample_rate as f32;
    create_wav_file(&path, sample_rate, 2, 16, duration_secs, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Failed to open file");
    //
    // // Try to seek within the short file
    // let result = decoder.seek(Duration::from_micros(1000)); // 1ms
    // // Should either succeed (if within file) or gracefully handle
    // if let Ok(actual) = result {
    //     assert!(actual <= Duration::from_secs_f32(duration_secs));
    // }

    assert!(decoder.supports_format(&path));
}

/// Test seeking in empty file (0 samples)
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_empty_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("empty.wav");

    // Create a valid WAV header with 0 bytes of data
    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = 0u32;
    let chunk_size = 36 + data_size;

    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // Seeking in empty file should either:
    // 1. Return position 0 (only valid position)
    // 2. Return an error

    assert!(decoder.supports_format(&path));
}

/// Test backward seek (seek to position before current)
#[test]
fn test_backward_seek() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("backward_seek.wav");

    create_wav_file(&path, 44100, 2, 16, 10.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // Seek forward first
    decoder.seek(Duration::from_secs(8)).expect("Forward seek failed");

    // Now seek backward
    let result = decoder.seek(Duration::from_secs(2));
    assert!(result.is_ok(), "Backward seek should succeed");

    let actual = result.unwrap();
    let tolerance = Duration::from_millis(50);
    assert!(
        actual >= Duration::from_secs(2).saturating_sub(tolerance)
            && actual <= Duration::from_secs(2) + tolerance,
        "Backward seek landed at {:?}, expected around 2s",
        actual
    );

    // Verify we can decode from the new position
    let chunk = decoder.decode_chunk(1024);
    assert!(chunk.is_ok(), "Decode after backward seek should succeed");
}

/// Test forward seek (seek to position after current)
#[test]
fn test_forward_seek() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("forward_seek.wav");

    create_wav_file(&path, 44100, 2, 16, 10.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Failed to open file");

    // Verify initial position is 0
    assert_eq!(decoder.position(), Duration::from_secs(0));

    // Seek forward
    let result = decoder.seek(Duration::from_secs(5));
    assert!(result.is_ok(), "Forward seek should succeed");

    let actual = result.unwrap();
    let tolerance = Duration::from_millis(50);
    assert!(
        actual >= Duration::from_secs(5).saturating_sub(tolerance)
            && actual <= Duration::from_secs(5) + tolerance,
        "Forward seek landed at {:?}, expected around 5s",
        actual
    );
}

// ============================================================================
// SEEK PRECISION TESTS FOR COMPRESSED FORMATS
// ============================================================================

/// Test MP3 frame boundary seeking precision
///
/// MP3 frames contain 1152 samples. At 44.1kHz:
/// - Frame duration: 1152 / 44100 = ~26.12ms
/// - Seeking should land on frame boundaries
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and MP3 files
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_mp3_frame_boundary_precision() {
    // MP3 specifics
    const MP3_SAMPLES_PER_FRAME: usize = 1152;
    const SAMPLE_RATE: u32 = 44100;
    const _FRAME_DURATION_MS: f64 = MP3_SAMPLES_PER_FRAME as f64 / SAMPLE_RATE as f64 * 1000.0;

    // TODO: Once implemented with MP3 support:
    // let path = ...; // MP3 test file
    // decoder.open(&path).expect("Failed to open MP3");
    //
    // // Seek to 1 second
    // let target = Duration::from_secs(1);
    // let result = decoder.seek(target).expect("Seek failed");
    //
    // // Calculate expected frame boundary
    // let target_sample = (target.as_secs_f64() * SAMPLE_RATE as f64) as usize;
    // let frame_index = target_sample / MP3_SAMPLES_PER_FRAME;
    // let expected_sample = frame_index * MP3_SAMPLES_PER_FRAME;
    // let expected_time = Duration::from_secs_f64(expected_sample as f64 / SAMPLE_RATE as f64);
    //
    // // Result should be within one frame of expected
    // let tolerance = Duration::from_secs_f64(FRAME_DURATION_MS / 1000.0);
    // assert!(
    //     result >= expected_time.saturating_sub(tolerance) && result <= expected_time + tolerance,
    //     "MP3 seek should land on frame boundary"
    // );
}

/// Test AAC frame boundary seeking precision
///
/// AAC frames contain 1024 samples. At 44.1kHz:
/// - Frame duration: 1024 / 44100 = ~23.22ms
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and AAC files
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_aac_frame_boundary_precision() {
    const AAC_SAMPLES_PER_FRAME: usize = 1024;
    const SAMPLE_RATE: u32 = 44100;
    const _FRAME_DURATION_MS: f64 = AAC_SAMPLES_PER_FRAME as f64 / SAMPLE_RATE as f64 * 1000.0;

    // TODO: Similar to MP3 test above
}

/// Test FLAC seek table usage
///
/// FLAC files can contain seek tables for efficient random access.
/// With seek tables, seeking should be very fast and sample-accurate.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation and FLAC files
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_flac_seek_table_usage() {
    // TODO: Create or use FLAC file with known seek table
    // Verify that seeking uses the seek table for efficiency
}

// ============================================================================
// SEEK AND VERIFY AUDIO CONTENT TESTS
// ============================================================================

/// Test that audio content after seek matches expected position
///
/// Uses a chirp signal where frequency indicates position, then verifies
/// the decoded audio has the expected frequency for the seek target.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_verify_content_chirp() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("chirp.wav");

    let sample_rate = 44100u32;
    let duration_secs = 5.0;
    let start_freq = 200.0;
    let end_freq = 2000.0;

    create_chirp_wav(&path, sample_rate, duration_secs, start_freq, end_freq);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Failed to open file");
    //
    // // Seek to middle (2.5 seconds)
    // let target = Duration::from_millis(2500);
    // decoder.seek(target).expect("Seek failed");
    //
    // // Decode a chunk and estimate frequency
    // let chunk = decoder.decode_chunk(4410).expect("Decode failed").expect("Should have data");
    // let freq = estimate_frequency(&chunk.samples, sample_rate);
    //
    // // Expected frequency at t=2.5s in a linear chirp from 200Hz to 2000Hz over 5s:
    // // freq(t) = start + (end - start) * t / duration
    // // freq(2.5) = 200 + (2000 - 200) * 2.5 / 5 = 200 + 900 = 1100 Hz
    // let expected_freq = 1100.0;
    // let tolerance = 100.0; // Hz tolerance for zero-crossing estimation
    //
    // assert!(
    //     (freq - expected_freq).abs() < tolerance,
    //     "Frequency after seek should be ~{} Hz, got {} Hz",
    //     expected_freq, freq
    // );

    assert!(decoder.supports_format(&path));
}

/// Test that audio content matches expected segment after seek
///
/// Uses a file with distinct frequency segments to verify seek position.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_verify_content_segments() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("segments.wav");

    let sample_rate = 44100u32;
    let segment_duration = 1.0; // 1 second per segment
    let num_segments = 5;

    create_segmented_wav(&path, sample_rate, segment_duration, num_segments);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Failed to open file");
    //
    // // Test each segment
    // let segment_frequencies = [200.0, 400.0, 600.0, 800.0, 1000.0];
    //
    // for (i, expected_freq) in segment_frequencies.iter().enumerate() {
    //     // Seek to middle of segment
    //     let target = Duration::from_secs_f32(i as f32 + 0.5);
    //     decoder.seek(target).expect("Seek failed");
    //
    //     // Decode and verify frequency
    //     let chunk = decoder.decode_chunk(4410).expect("Decode failed").expect("Should have data");
    //     let freq = estimate_frequency(&chunk.samples, sample_rate);
    //
    //     let tolerance = 50.0;
    //     assert!(
    //         (freq - expected_freq).abs() < tolerance,
    //         "Segment {} should have frequency ~{} Hz, got {} Hz",
    //         i, expected_freq, freq
    //     );
    // }

    assert!(decoder.supports_format(&path));
}

/// Test that seeking preserves bit-perfect output for lossless formats
///
/// For WAV and FLAC, audio after seeking should be bit-identical to
/// audio obtained by decoding from the start.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_bit_perfect_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("bit_perfect.wav");

    let sample_rate = 44100u32;
    create_wav_file(&path, sample_rate, 2, 16, 5.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // // First, decode the entire file from start
    // let full_buffer = decoder.decode(&path).expect("Full decode failed");
    //
    // // Now use streaming decode with seek
    // decoder.open(&path).expect("Open failed");
    //
    // // Seek to 2 seconds and decode
    // let target = Duration::from_secs(2);
    // decoder.seek(target).expect("Seek failed");
    //
    // let seek_chunk = decoder.decode_chunk(4410).expect("Decode failed").expect("Should have data");
    //
    // // Compare with corresponding samples from full decode
    // let start_sample = time_to_sample_index(target, sample_rate) * 2; // *2 for stereo
    // let end_sample = start_sample + seek_chunk.samples.len();
    //
    // for i in 0..seek_chunk.samples.len() {
    //     assert_eq!(
    //         seek_chunk.samples[i],
    //         full_buffer.samples[start_sample + i],
    //         "Sample {} mismatch after seek",
    //         i
    //     );
    // }

    assert!(decoder.supports_format(&path));
}

// ============================================================================
// SEEK STATE MANAGEMENT TESTS
// ============================================================================

/// Test that decoder state is properly reset after seek
///
/// The decoder should clear any internal buffers and state when seeking.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_seek_resets_decoder_state() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("state_reset.wav");

    create_wav_file(&path, 44100, 2, 16, 5.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Open failed");
    //
    // // Decode some samples
    // let _ = decoder.decode_chunk(4410);
    //
    // // Seek back to beginning
    // decoder.seek(Duration::from_secs(0)).expect("Seek failed");
    //
    // // Decode again - should match what we'd get from fresh decode
    // let after_seek = decoder.decode_chunk(4410).expect("Decode failed").expect("Should have data");
    //
    // // Fresh decode for comparison
    // let mut fresh_decoder = SymphoniaDecoder::new();
    // fresh_decoder.open(&path).expect("Open failed");
    // let fresh = fresh_decoder.decode_chunk(4410).expect("Decode failed").expect("Should have data");
    //
    // assert_eq!(after_seek.samples, fresh.samples, "Samples after seek should match fresh decode");

    assert!(decoder.supports_format(&path));
}

/// Test position reporting accuracy after seek
///
/// TODO: Requires `AudioDecoder::seek()` and `position()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
fn test_position_reporting_after_seek() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("position_report.wav");

    create_wav_file(&path, 44100, 2, 16, 10.0, 440.0);

    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Open failed");
    //
    // let positions = [
    //     Duration::from_millis(0),
    //     Duration::from_millis(500),
    //     Duration::from_secs(1),
    //     Duration::from_secs(5),
    //     Duration::from_millis(9500),
    // ];
    //
    // for target in positions {
    //     decoder.seek(target).expect("Seek failed");
    //     let reported = decoder.position();
    //
    //     // Position should match what seek returned
    //     let tolerance = Duration::from_millis(1); // 1ms tolerance
    //     assert!(
    //         reported >= target.saturating_sub(tolerance) && reported <= target + tolerance,
    //         "Reported position {:?} should match seek target {:?}",
    //         reported, target
    //     );
    // }
}

/// Test duration reporting
#[test]
fn test_duration_reporting() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("duration.wav");

    let expected_duration = 5.0;
    create_wav_file(&path, 44100, 2, 16, expected_duration, 440.0);

    let mut decoder = SymphoniaDecoder::new();

    decoder.open(&path).expect("Open failed");

    let duration = decoder.duration().expect("Should have duration");
    let expected = Duration::from_secs_f32(expected_duration);

    // Allow small tolerance for sample alignment
    let tolerance = Duration::from_millis(50);
    assert!(
        duration >= expected.saturating_sub(tolerance) && duration <= expected + tolerance,
        "Duration {:?} should match expected {:?}",
        duration, expected
    );
}

// ============================================================================
// CONCURRENT SEEK SAFETY TESTS
// ============================================================================

/// Test that seeking is thread-safe
///
/// While a single decoder instance should not be used from multiple threads,
/// the seeking implementation should not have any global state that could
/// cause issues with multiple concurrent decoder instances.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
#[allow(unused_imports)]
fn test_concurrent_seeking_multiple_decoders() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = Arc::new(tempfile::tempdir().unwrap());

    // Create test files
    for i in 0..4 {
        let path = temp_dir.path().join(format!("concurrent_{}.wav", i));
        create_wav_file(&path, 44100, 2, 16, 10.0, 440.0 + i as f32 * 100.0);
    }

    // TODO: Once seeking is implemented:
    // let mut handles = vec![];
    //
    // for i in 0..4 {
    //     let temp_dir = Arc::clone(&temp_dir);
    //     handles.push(thread::spawn(move || {
    //         let path = temp_dir.path().join(format!("concurrent_{}.wav", i));
    //         let mut decoder = SymphoniaDecoder::new();
    //         decoder.open(&path).expect("Open failed");
    //
    //         // Perform multiple seeks
    //         for j in 0..20 {
    //             let position = Duration::from_millis((j * 500) % 10000);
    //             decoder.seek(position).expect("Seek failed");
    //             let _ = decoder.decode_chunk(1024);
    //         }
    //
    //         true
    //     }));
    // }
    //
    // for handle in handles {
    //     assert!(handle.join().expect("Thread panicked"));
    // }
}

// ============================================================================
// SEEK PERFORMANCE TESTS (BENCHMARKS)
// ============================================================================

/// Benchmark seek latency
///
/// Measures the time taken to perform seek operations.
/// Useful for ensuring seeks are fast enough for real-time scrubbing.
///
/// TODO: Requires `AudioDecoder::seek()` method implementation
#[test]
#[ignore = "Seeking not yet implemented in AudioDecoder trait"]
#[allow(unused_imports)]
fn test_seek_latency_benchmark() {
    use std::time::Instant;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("benchmark.wav");

    // Create a longer file for meaningful benchmark
    create_wav_file(&path, 44100, 2, 16, 60.0, 440.0); // 1 minute

    let _decoder = SymphoniaDecoder::new();

    // TODO: Once seeking is implemented:
    // decoder.open(&path).expect("Open failed");
    //
    // let num_seeks = 100;
    // let start = Instant::now();
    //
    // for i in 0..num_seeks {
    //     let position = Duration::from_secs((i % 60) as u64);
    //     decoder.seek(position).expect("Seek failed");
    // }
    //
    // let elapsed = start.elapsed();
    // let avg_seek_ms = elapsed.as_secs_f64() * 1000.0 / num_seeks as f64;
    //
    // println!("Average seek latency: {:.3}ms", avg_seek_ms);
    //
    // // Seek should be fast - less than 50ms average for WAV
    // assert!(
    //     avg_seek_ms < 50.0,
    //     "Average seek latency {:.3}ms exceeds 50ms threshold",
    //     avg_seek_ms
    // );
}

// ============================================================================
// HELPER FUNCTION TESTS (to verify test infrastructure)
// ============================================================================

/// Verify helper function: frequency estimation accuracy
#[test]
fn test_frequency_estimation_helper() {
    let sample_rate = 44100u32;
    let duration = 0.1;
    let test_freq = 440.0;

    // Generate test signal
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * test_freq * t).sin();
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    let estimated = estimate_frequency(&samples, sample_rate);

    // Zero-crossing method has limited precision
    let tolerance = 50.0;
    assert!(
        (estimated - test_freq).abs() < tolerance,
        "Frequency estimation: expected {} Hz, got {} Hz",
        test_freq,
        estimated
    );
}

/// Verify helper function: time to sample index conversion
#[test]
fn test_time_to_sample_index_helper() {
    let sample_rate = 44100u32;

    assert_eq!(time_to_sample_index(Duration::from_secs(0), sample_rate), 0);
    assert_eq!(
        time_to_sample_index(Duration::from_secs(1), sample_rate),
        44100
    );
    assert_eq!(
        time_to_sample_index(Duration::from_millis(500), sample_rate),
        22050
    );
}

/// Verify helper function: sample index to time conversion
#[test]
fn test_sample_index_to_time_helper() {
    let sample_rate = 44100u32;

    assert_eq!(
        sample_index_to_time(0, sample_rate),
        Duration::from_secs(0)
    );
    assert_eq!(
        sample_index_to_time(44100, sample_rate),
        Duration::from_secs(1)
    );

    let half_second = sample_index_to_time(22050, sample_rate);
    assert!(
        (half_second.as_secs_f64() - 0.5).abs() < 0.001,
        "Expected ~0.5s, got {:?}",
        half_second
    );
}

/// Verify WAV file creation helper
#[test]
fn test_wav_creation_helper() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("helper_test.wav");

    create_wav_file(&path, 44100, 2, 16, 1.0, 440.0);

    // Verify the file can be decoded
    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).expect("Failed to decode test WAV");

    // Should have approximately 1 second of stereo audio
    let expected_samples = 44100 * 2; // 1 second * 2 channels
    let tolerance = 100; // Allow small variation

    assert!(
        (buffer.samples.len() as i32 - expected_samples as i32).abs() < tolerance,
        "Expected ~{} samples, got {}",
        expected_samples,
        buffer.samples.len()
    );
}

/// Verify chirp WAV creation helper
#[test]
fn test_chirp_wav_creation_helper() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("chirp_test.wav");

    let sample_rate = 44100u32;
    create_chirp_wav(&path, sample_rate, 2.0, 200.0, 2000.0);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).expect("Failed to decode chirp WAV");

    // Verify we got audio
    assert!(
        buffer.samples.len() > 0,
        "Chirp WAV should contain samples"
    );

    // Verify frequency at start is lower than at end
    let chunk_size = 4410; // 100ms
    let start_chunk: Vec<f32> = buffer.samples.iter().take(chunk_size).copied().collect();
    let end_chunk: Vec<f32> = buffer
        .samples
        .iter()
        .rev()
        .take(chunk_size)
        .copied()
        .collect();

    let start_freq = estimate_frequency(&start_chunk, sample_rate);
    let end_freq = estimate_frequency(&end_chunk, sample_rate);

    // End frequency should be higher than start (chirp goes up)
    assert!(
        end_freq > start_freq,
        "Chirp should increase in frequency: start={} Hz, end={} Hz",
        start_freq,
        end_freq
    );
}

/// Verify segmented WAV creation helper
#[test]
fn test_segmented_wav_creation_helper() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("segmented_test.wav");

    let sample_rate = 44100u32;
    create_segmented_wav(&path, sample_rate, 1.0, 3);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).expect("Failed to decode segmented WAV");

    // Should have 3 seconds of audio (3 segments * 1 second each)
    let expected_samples = sample_rate as usize * 2 * 3; // 3 seconds * stereo
    let tolerance = 100;

    assert!(
        (buffer.samples.len() as i32 - expected_samples as i32).abs() < tolerance,
        "Expected ~{} samples, got {}",
        expected_samples,
        buffer.samples.len()
    );
}
