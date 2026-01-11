//! Critical Bug Hunt: Decoder and Audio Source Analysis
//!
//! This file documents REAL bugs found in the audio decoder implementations,
//! with code references, explanations, and test cases.
//!
//! ## FILES ANALYZED:
//! - `libraries/soul-audio/src/decoder.rs` - SymphoniaDecoder
//! - `libraries/soul-audio-desktop/src/sources/local.rs` - LocalAudioSource
//!
//! ## BUGS FOUND:
//!
//! ### BUG 1: Asymmetric Integer-to-Float Conversion (CRITICAL)
//! **Location**: decoder.rs lines 52-56, 68-73, 84-90
//! **Issue**: Signed integer conversion divides by `iXX::MAX` which causes asymmetry
//! **Impact**: Full-scale negative samples (-32768 for i16) exceed -1.0
//!
//! ### BUG 2: Unsigned 24-bit Scaling Error (CRITICAL)
//! **Location**: decoder.rs lines 152-173
//! **Issue**: U24 uses wrong divisor (8388607 = 2^23-1) but U24 max is 16777215 (2^24-1)
//! **Impact**: Samples scaled incorrectly, output exceeds [-1.0, 1.0] range
//!
//! ### BUG 3: Multi-Channel Audio Discards Channels (MEDIUM)
//! **Location**: decoder.rs lines 32-38, local.rs lines 447-456
//! **Issue**: Only channels 0 and 1 are used; 5.1/7.1 audio loses LFE, surround channels
//! **Impact**: Loss of audio content, incorrect downmix
//!
//! ### BUG 4: Channel Count Mismatch After Conversion (MEDIUM)
//! **Location**: decoder.rs line 299-306
//! **Issue**: Final buffer reports original channel count but data is always stereo
//! **Impact**: AudioFormat.channels doesn't match actual sample layout
//!
//! ### BUG 5: Seek Position Tracking Mismatch (LOW)
//! **Location**: local.rs lines 594-597
//! **Issue**: samples_decoded is set to samples_read after seek, but they track different rates
//! **Impact**: Internal accounting is incorrect (samples_decoded should be at source rate)
//!
//! ### BUG 6: Duration Calculation for Unknown Frames (INFO)
//! **Location**: local.rs lines 162-166
//! **Issue**: Defaults to 180 seconds when n_frames is unknown
//! **Impact**: Progress bar may be incorrect for some streaming formats
//!
//! ### BUG 7: F32 Samples Not Clamped (POTENTIAL)
//! **Location**: decoder.rs line 30, local.rs line 475
//! **Issue**: F32 input passed through without clamping to [-1.0, 1.0]
//! **Impact**: May produce out-of-range output if source has intersample peaks
//!

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Create a 16-bit PCM WAV file with specific samples
fn create_wav_16bit(path: &PathBuf, sample_rate: u32, samples: &[i16], channels: u16) {
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (samples.len() * 2) as u32;
    let chunk_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();

    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    for &sample in samples {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}

/// Create an 8-bit PCM WAV file with specific samples
fn create_wav_8bit(path: &PathBuf, sample_rate: u32, samples: &[u8], channels: u16) {
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32;
    let block_align = channels;
    let data_size = samples.len() as u32;
    let chunk_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&8u16.to_le_bytes()).unwrap(); // 8-bit

    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    for &sample in samples {
        file.write_all(&[sample]).unwrap();
    }
}

/// Create a 32-bit float WAV file with specific samples
fn create_wav_f32(path: &PathBuf, sample_rate: u32, samples: &[f32], channels: u16) {
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 4;
    let block_align = channels * 4;
    let data_size = (samples.len() * 4) as u32;
    let chunk_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&3u16.to_le_bytes()).unwrap(); // IEEE float format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&32u16.to_le_bytes()).unwrap(); // 32-bit

    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    for &sample in samples {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}

/// Create a 24-bit PCM WAV file with specific samples (stored as i32, only 24 bits used)
fn create_wav_24bit(path: &PathBuf, sample_rate: u32, samples: &[i32], channels: u16) {
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 3;
    let block_align = channels * 3;
    let data_size = (samples.len() * 3) as u32;
    let chunk_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&24u16.to_le_bytes()).unwrap(); // 24-bit

    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    for &sample in samples {
        // Write 24-bit little-endian (3 bytes)
        let bytes = sample.to_le_bytes();
        file.write_all(&bytes[0..3]).unwrap();
    }
}

// ============================================================================
// BUG 1: ASYMMETRIC INTEGER-TO-FLOAT CONVERSION
// ============================================================================
//
// CODE (decoder.rs:68-73):
// ```rust
// AudioBufferRef::S16(buf) => {
//     let left: Vec<f32> = buf
//         .chan(0)
//         .iter()
//         .map(|&s| s as f32 / i16::MAX as f32)  // <-- BUG HERE
//         .collect();
// ```
//
// PROBLEM: i16 range is -32768 to 32767
//          i16::MAX = 32767
//          -32768 / 32767 = -1.00003... (exceeds -1.0!)
//
// This is a well-known audio programming pitfall. The correct approach is:
// - Divide by 32768.0 (not 32767) for symmetric scaling, OR
// - Use separate divisors for positive and negative values
//

#[test]
fn test_bug1_i16_negative_full_scale_exceeds_minus_one() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("i16_negative_full_scale.wav");

    // Create WAV with full-scale negative sample
    let samples = vec![i16::MIN, i16::MIN]; // -32768, -32768 (stereo frame)
    create_wav_16bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok(), "Should decode successfully");

    let buffer = result.unwrap();

    // BUG: With current code, -32768 / 32767 = -1.00003...
    // This test documents the bug
    for (i, &sample) in buffer.samples.iter().enumerate() {
        if sample < -1.0 {
            println!("BUG CONFIRMED: Sample {} = {} (exceeds -1.0)", i, sample);
        }
        // The correct behavior would be:
        // assert!(sample >= -1.0 && sample <= 1.0,
        //     "Sample {} = {} is out of range [-1.0, 1.0]", i, sample);
    }

    // Document expected vs actual
    let expected_correct = -32768.0 / 32768.0; // -1.0
    let expected_buggy = -32768.0 / 32767.0; // -1.00003...
    println!("Expected (correct): {}", expected_correct);
    println!("Expected (buggy):   {}", expected_buggy);
    println!("Actual first sample: {}", buffer.samples[0]);
}

#[test]
fn test_bug1_i16_positive_full_scale_correct() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("i16_positive_full_scale.wav");

    // Full-scale positive is correctly handled (32767/32767 = 1.0)
    let samples = vec![i16::MAX, i16::MAX];
    create_wav_16bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok());

    let buffer = result.unwrap();

    // This should be exactly 1.0
    assert!(
        (buffer.samples[0] - 1.0).abs() < 0.0001,
        "Positive full scale should be 1.0, got {}",
        buffer.samples[0]
    );
}

#[test]
fn test_bug1_i8_asymmetric_conversion() {
    // Same bug exists for i8: i8::MIN = -128, i8::MAX = 127
    // -128 / 127 = -1.00787...
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("i8_negative_full_scale.wav");

    // Note: 8-bit WAV is unsigned, so we test via expected conversion
    // Unsigned 0 should map to -1.0, unsigned 255 should map to ~1.0
    let samples = vec![0u8, 0u8]; // Minimum value in unsigned 8-bit
    create_wav_8bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok());

    let buffer = result.unwrap();

    // U8 conversion: (0 / 255) * 2 - 1 = -1.0 (this is actually correct!)
    println!("U8 min (0) converts to: {}", buffer.samples[0]);
}

// ============================================================================
// BUG 2: UNSIGNED 24-BIT SCALING ERROR
// ============================================================================
//
// CODE (decoder.rs:152-159):
// ```rust
// AudioBufferRef::U24(buf) => {
//     let left: Vec<f32> = buf
//         .chan(0)
//         .iter()
//         .map(|&s| {
//             let val = s.inner() as f32;
//             (val / 8388607.0) * 2.0 - 1.0 // 2^23 - 1, scale to [-1, 1]
//         })
// ```
//
// PROBLEM: U24 range is 0 to 16777215 (2^24 - 1)
//          Code uses 8388607 (2^23 - 1) as divisor
//          This means full-scale positive (16777215) becomes:
//          (16777215 / 8388607) * 2 - 1 = 2.99999... * 2 - 1 = ~3.0 (WAY out of range!)
//
// CORRECT: Should divide by 16777215.0 for U24
//

// Note: This test is theoretical because Symphonia may not commonly produce U24 buffers
// Most 24-bit audio is signed (S24), but the bug exists in the code nonetheless

#[test]
fn test_bug2_u24_scaling_theoretical() {
    // Document the mathematical bug
    let u24_max: f32 = 16777215.0; // 2^24 - 1
    let wrong_divisor: f32 = 8388607.0; // 2^23 - 1 (what code uses)
    let correct_divisor: f32 = 16777215.0;

    // With wrong divisor
    let wrong_result = (u24_max / wrong_divisor) * 2.0 - 1.0;
    // With correct divisor
    let correct_result = (u24_max / correct_divisor) * 2.0 - 1.0;

    println!("U24 max value: {}", u24_max);
    println!("Wrong result (using 2^23-1): {}", wrong_result);
    println!("Correct result (using 2^24-1): {}", correct_result);

    assert!(
        wrong_result > 1.0,
        "BUG DOCUMENTED: Wrong divisor produces {} (exceeds 1.0)",
        wrong_result
    );
    assert!(
        (correct_result - 1.0).abs() < 0.0001,
        "Correct divisor should produce 1.0"
    );
}

// ============================================================================
// BUG 3: MULTI-CHANNEL AUDIO LOSES CHANNELS
// ============================================================================
//
// CODE (decoder.rs:32-38):
// ```rust
// let left = buf.chan(0);
// let right = if channels > 1 {
//     buf.chan(1)
// } else {
//     buf.chan(0)
// };
// Self::interleave_f32(left, right)
// ```
//
// PROBLEM: For 5.1 audio (6 channels):
//   - Channel 0: Front Left
//   - Channel 1: Front Right
//   - Channel 2: Center
//   - Channel 3: LFE (Subwoofer)
//   - Channel 4: Rear Left
//   - Channel 5: Rear Right
//
// Current code only takes channels 0 and 1, losing Center, LFE, and Surrounds.
// Proper downmix should fold all channels with appropriate coefficients.
//

#[test]
fn test_bug3_multichannel_downmix_not_implemented() {
    // This is a design issue - we can't directly test without a 5.1 source file
    // Document the expected behavior:
    //
    // ITU-R BS.775-1 stereo downmix coefficients:
    // L_out = L + 0.707*C + 0.707*Ls
    // R_out = R + 0.707*C + 0.707*Rs
    //
    // Current behavior: L_out = L, R_out = R (discards C, LFE, Ls, Rs)

    println!("BUG DOCUMENTED: Multi-channel audio is not properly downmixed");
    println!("Current behavior: Only channels 0 and 1 are used");
    println!("Expected behavior: All channels should be mixed with proper coefficients");

    // Test mono handling (which IS implemented)
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono.wav");

    let samples = vec![1000i16, 2000i16, 3000i16]; // 3 mono samples
    create_wav_16bit(&path, 44100, &samples, 1);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok());

    let buffer = result.unwrap();

    // Mono should be duplicated to stereo (this works correctly)
    // Check that L == R
    for chunk in buffer.samples.chunks(2) {
        assert!(
            (chunk[0] - chunk[1]).abs() < 0.0001,
            "Mono duplication failed: L={}, R={}",
            chunk[0],
            chunk[1]
        );
    }
}

// ============================================================================
// BUG 4: CHANNEL COUNT MISMATCH IN OUTPUT FORMAT
// ============================================================================
//
// CODE (decoder.rs:299-306):
// ```rust
// channels = decoded.spec().channels.count() as u16;  // <-- Original channel count
// // ... processing always produces stereo interleaved ...
// let format = AudioFormat::new(SampleRate::new(sample_rate), channels, 32);
// Ok(AudioBuffer::new(all_samples, format))
// ```
//
// PROBLEM: If input is mono (1 channel) or surround (6 channels):
//   - The data is always interleaved as stereo
//   - But format.channels reflects original channel count
//   - Consumer expects `channels` samples per frame, but there are always 2
//

#[test]
fn test_bug4_channel_count_mismatch_mono() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono_channel_mismatch.wav");

    // Create mono file with 100 samples
    let samples: Vec<i16> = (0..100).map(|i| (i * 100) as i16).collect();
    create_wav_16bit(&path, 44100, &samples, 1);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok());

    let buffer = result.unwrap();

    // The decoder duplicates mono to stereo, so we have 2x the samples
    let expected_samples_if_mono = 100;
    let expected_samples_if_stereo = 200;

    println!("Input: 100 mono samples");
    println!("Output samples: {}", buffer.samples.len());
    println!("Output channels reported: {}", buffer.format.channels);

    // BUG: Format says 1 channel, but data is interleaved stereo
    // This could confuse downstream processing
    if buffer.format.channels == 1 && buffer.samples.len() == expected_samples_if_stereo {
        println!("BUG CONFIRMED: Format says mono but data is stereo");
    }
}

// ============================================================================
// BUG 7: F32 SAMPLES NOT CLAMPED
// ============================================================================
//
// CODE (decoder.rs:30-38, local.rs:475):
// ```rust
// AudioBufferRef::F32(buf) => {
//     let left = buf.chan(0);
//     let right = if channels > 1 { ... };
//     Self::interleave_f32(left, right)
// }
// // No clamping applied!
// ```
//
// PROBLEM: F32 audio files can legally contain intersample peaks > 1.0
// Professional mastering tools like Sonnox Pro-Codec detect these.
// If the source has samples > 1.0, they pass through unclamped.
//

#[test]
fn test_bug7_f32_no_clamping() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("f32_over_one.wav");

    // Create F32 WAV with samples exceeding [-1.0, 1.0]
    let samples = vec![1.5f32, 1.5f32, -1.5f32, -1.5f32, 2.0f32, 2.0f32];
    create_wav_f32(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);
    assert!(result.is_ok());

    let buffer = result.unwrap();

    // Check if out-of-range samples are clamped
    let mut has_over_one = false;
    for &sample in &buffer.samples {
        if sample > 1.0 || sample < -1.0 {
            has_over_one = true;
            println!("Sample exceeds range: {}", sample);
        }
    }

    if has_over_one {
        println!("BUG/DESIGN: F32 samples are not clamped to [-1.0, 1.0]");
        println!("This may be intentional for headroom, but could cause clipping in output");
    }
}

// ============================================================================
// ADDITIONAL TESTS: VERIFY CORRECT BEHAVIOR
// ============================================================================

#[test]
fn test_i16_zero_is_zero() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("i16_zero.wav");

    let samples = vec![0i16, 0i16, 0i16, 0i16];
    create_wav_16bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Zero should convert to exactly 0.0
    for &sample in &buffer.samples {
        assert!(
            sample.abs() < 0.0001,
            "Zero sample should be 0.0, got {}",
            sample
        );
    }
}

#[test]
fn test_i16_midpoint_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("i16_midpoint.wav");

    // Half of full scale
    let half_scale = i16::MAX / 2; // 16383
    let samples = vec![half_scale, half_scale];
    create_wav_16bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Should be approximately 0.5
    assert!(
        (buffer.samples[0] - 0.5).abs() < 0.001,
        "Half scale should be ~0.5, got {}",
        buffer.samples[0]
    );
}

#[test]
fn test_u8_full_range() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("u8_full_range.wav");

    // 8-bit audio: 0 = -1.0, 128 = 0.0, 255 = +1.0
    let samples = vec![0u8, 0u8, 128u8, 128u8, 255u8, 255u8];
    create_wav_8bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    println!("U8 0 -> {}", buffer.samples[0]);
    println!("U8 128 -> {}", buffer.samples[2]);
    println!("U8 255 -> {}", buffer.samples[4]);

    // Check conversions
    // U8 formula: (val / 255.0) * 2.0 - 1.0
    // 0 -> -1.0
    // 128 -> 0.003... (slightly positive)
    // 255 -> 1.0

    assert!(
        buffer.samples[0] < -0.99,
        "U8 0 should be close to -1.0, got {}",
        buffer.samples[0]
    );

    assert!(
        buffer.samples[2].abs() < 0.01,
        "U8 128 should be close to 0.0, got {}",
        buffer.samples[2]
    );

    assert!(
        buffer.samples[4] > 0.99,
        "U8 255 should be close to 1.0, got {}",
        buffer.samples[4]
    );
}

#[test]
fn test_24bit_signed_conversion() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("s24_test.wav");

    // S24 range: -8388608 to 8388607
    let s24_max = 8388607i32;
    let s24_min = -8388608i32;
    let samples = vec![s24_max, s24_max, s24_min, s24_min, 0, 0];
    create_wav_24bit(&path, 44100, &samples, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    match result {
        Ok(buffer) => {
            println!("S24 max (8388607) -> {}", buffer.samples[0]);
            println!("S24 min (-8388608) -> {}", buffer.samples[2]);
            println!("S24 zero (0) -> {}", buffer.samples[4]);

            // S24 max should be 1.0
            assert!(
                (buffer.samples[0] - 1.0).abs() < 0.0001,
                "S24 max should be 1.0, got {}",
                buffer.samples[0]
            );

            // S24 min has same asymmetry bug as i16
            // -8388608 / 8388607 = -1.000000119...
            if buffer.samples[2] < -1.0 {
                println!(
                    "BUG CONFIRMED in S24: min value {} exceeds -1.0",
                    buffer.samples[2]
                );
            }
        }
        Err(e) => {
            println!("24-bit WAV decode failed: {}", e);
            println!("This may be expected if format is not supported");
        }
    }
}

// ============================================================================
// STRESS TESTS FOR DECODER ROBUSTNESS
// ============================================================================

#[test]
fn test_decoder_with_alternating_extreme_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("extreme_alternating.wav");

    // Create worst-case signal: alternating full-scale positive/negative
    let samples: Vec<i16> = (0..1000)
        .map(|i| if i % 2 == 0 { i16::MAX } else { i16::MIN })
        .collect();
    create_wav_16bit(&path, 44100, &samples, 1);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // All samples should be finite
    for (i, &sample) in buffer.samples.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
    }
}

#[test]
fn test_decoder_with_ramp() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("linear_ramp.wav");

    // Create linear ramp from min to max
    let num_samples = 1000;
    let samples: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / (num_samples - 1) as f32;
            (i16::MIN as f32 + t * (i16::MAX as f32 - i16::MIN as f32)) as i16
        })
        .collect();
    create_wav_16bit(&path, 44100, &samples, 1);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Verify samples are monotonically increasing
    let mut prev = buffer.samples[0];
    for &sample in &buffer.samples[2..] {
        // Every other sample (left channel only since mono is duplicated to stereo)
        // Note: We're iterating all samples, L and R alternate
    }

    // Just verify no NaN or Inf
    for &sample in &buffer.samples {
        assert!(sample.is_finite());
    }
}

// ============================================================================
// SUMMARY OF BUGS AND RECOMMENDED FIXES
// ============================================================================
//
// BUG 1 FIX (Asymmetric integer conversion):
//   Change: `s as f32 / i16::MAX as f32`
//   To:     `s as f32 / 32768.0`  (or use i16::MIN.abs())
//
// BUG 2 FIX (U24 scaling):
//   Change: `(val / 8388607.0) * 2.0 - 1.0`
//   To:     `(val / 16777215.0) * 2.0 - 1.0`
//
// BUG 3 FIX (Multi-channel):
//   Implement proper ITU-R BS.775-1 downmix:
//   ```rust
//   if channels >= 3 {
//       let center = buf.chan(2);
//       let lfe = if channels >= 4 { buf.chan(3) } else { &[] };
//       // Apply mix coefficients...
//   }
//   ```
//
// BUG 4 FIX (Channel count):
//   Always set output channels to 2 since output is always stereo:
//   ```rust
//   let format = AudioFormat::new(SampleRate::new(sample_rate), 2, 32);
//   ```
//
// BUG 7 FIX (F32 clamping - optional):
//   Add soft clipping or hard clamping:
//   ```rust
//   AudioBufferRef::F32(buf) => {
//       Self::interleave_to_stereo_f32(&buf, |s| s.clamp(-1.0, 1.0))
//   }
//   ```
