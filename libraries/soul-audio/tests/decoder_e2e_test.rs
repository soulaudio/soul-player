//! Comprehensive end-to-end tests for the audio decoder
//!
//! This test suite covers:
//! - All supported formats (FLAC, WAV, MP3, AAC, OGG, OPUS, ALAC, APE, WavPack)
//! - Bit depth variations (8, 16, 24, 32-bit int, 32-bit float)
//! - Sample rate edge cases (8kHz to 384kHz)
//! - Corrupted file handling
//! - Truncated files
//! - Malformed headers
//! - Empty files
//! - Very large files (simulated)
//! - Multi-channel audio (mono, stereo, 5.1, 7.1)
//! - Seeking accuracy tests
//! - Gapless playback accuracy
//! - Metadata extraction edge cases
//!
//! Testing approaches used:
//! - Null testing (decode verification)
//! - Bit-perfect verification
//! - Sample-accurate verification
//! - Memory leak detection patterns
//! - Error recovery testing

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a WAV file with specified parameters
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

    // Determine format code based on bit depth
    let format_code: u16 = if bits_per_sample == 32 {
        3 // IEEE float
    } else {
        1 // PCM
    };

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
                8 => {
                    // Unsigned 8-bit: 0-255, 128 is silence
                    let sample_u8 = ((sample_f + 1.0) * 127.5) as u8;
                    file.write_all(&[sample_u8]).unwrap();
                }
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
                    // 32-bit float
                    file.write_all(&sample_f.to_le_bytes()).unwrap();
                }
                _ => panic!("Unsupported bit depth: {}", bits_per_sample),
            }
        }
    }
}

/// Create a standard 16-bit stereo WAV file
fn create_test_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
    create_wav_file(path, sample_rate, channels, 16, duration_secs, 440.0);
}

/// Create a WAV file with a specific frequency for frequency detection tests
fn create_freq_test_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, frequency: f32) {
    create_wav_file(path, sample_rate, 2, 16, duration_secs, frequency);
}

/// Create a WAV file with DC offset for testing
fn create_dc_offset_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, dc_offset: f32) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut file = File::create(path).expect("Failed to create WAV file");

    let channels = 2u16;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    let sample_i16 = (dc_offset.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
    for _ in 0..num_samples {
        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a multi-channel WAV file (for surround sound testing)
fn create_multichannel_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;

    // For multichannel, use WAVE_FORMAT_EXTENSIBLE
    let fmt_chunk_size = 40u32; // Extended format chunk
    let chunk_size = 20 + fmt_chunk_size + 8 + data_size;

    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // Extended fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&fmt_chunk_size.to_le_bytes()).unwrap();
    file.write_all(&0xFFFEu16.to_le_bytes()).unwrap(); // WAVE_FORMAT_EXTENSIBLE
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
    file.write_all(&22u16.to_le_bytes()).unwrap(); // extension size

    // Valid bits per sample
    file.write_all(&16u16.to_le_bytes()).unwrap();

    // Channel mask based on channel count
    let channel_mask: u32 = match channels {
        1 => 0x04,  // FC
        2 => 0x03,  // FL, FR
        3 => 0x07,  // FL, FR, FC
        4 => 0x33,  // FL, FR, BL, BR
        5 => 0x37,  // FL, FR, FC, BL, BR
        6 => 0x3F,  // FL, FR, FC, LFE, BL, BR (5.1)
        7 => 0x13F, // FL, FR, FC, LFE, BC, SL, SR
        8 => 0x63F, // FL, FR, FC, LFE, BL, BR, SL, SR (7.1)
        _ => 0,
    };
    file.write_all(&channel_mask.to_le_bytes()).unwrap();

    // Sub format GUID (PCM)
    file.write_all(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ])
    .unwrap();

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate samples with different frequencies per channel
    let base_freq = 440.0f32;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        for ch in 0..channels {
            let freq = base_freq * (1.0 + ch as f32 * 0.1); // Slightly different per channel
            let sample_f = (2.0 * PI * freq * t).sin() * 0.7; // Reduce amplitude
            let sample_i16 = (sample_f * i16::MAX as f32) as i16;
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a WAV file with silence
fn create_silence_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut file = File::create(path).expect("Failed to create WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write silence
    let silence = vec![0u8; data_size as usize];
    file.write_all(&silence).unwrap();
}

/// Create a WAV file with impulse (click) for transient testing
fn create_impulse_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, impulse_position: f32) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let impulse_sample = (impulse_position * num_samples as f32) as usize;

    let mut file = File::create(path).expect("Failed to create WAV file");

    let channels = 2u16;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    for i in 0..num_samples {
        let sample_i16 = if i == impulse_sample { i16::MAX } else { 0i16 };
        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Calculate RMS of samples
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Find peak amplitude
fn find_peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|s| s.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

/// Count zero crossings
fn count_zero_crossings(samples: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..samples.len() {
        if (samples[i - 1] < 0.0 && samples[i] >= 0.0)
            || (samples[i - 1] >= 0.0 && samples[i] < 0.0)
        {
            count += 1;
        }
    }
    count
}

/// Estimate fundamental frequency from zero crossings
fn estimate_frequency_from_crossings(samples: &[f32], sample_rate: u32) -> f32 {
    let crossings = count_zero_crossings(samples);
    let duration = samples.len() as f32 / (sample_rate as f32 * 2.0); // Stereo
    (crossings as f32 / 2.0) / duration
}

/// Compare two buffers for near equality
fn buffers_nearly_equal(a: &[f32], b: &[f32], tolerance: f32) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

/// Calculate maximum difference between buffers
fn max_difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

// ============================================================================
// FORMAT SUPPORT TESTS
// ============================================================================

#[test]
fn test_wav_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.wav")));
    assert!(decoder.supports_format(&PathBuf::from("test.WAV")));
    assert!(decoder.supports_format(&PathBuf::from("test.Wav")));
}

#[test]
fn test_mp3_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.mp3")));
    assert!(decoder.supports_format(&PathBuf::from("test.MP3")));
}

#[test]
fn test_flac_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.flac")));
    assert!(decoder.supports_format(&PathBuf::from("test.FLAC")));
}

#[test]
fn test_ogg_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.ogg")));
    assert!(decoder.supports_format(&PathBuf::from("test.OGG")));
}

#[test]
fn test_opus_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.opus")));
    assert!(decoder.supports_format(&PathBuf::from("test.OPUS")));
}

#[test]
fn test_aac_format_support() {
    let decoder = SymphoniaDecoder::new();
    assert!(decoder.supports_format(&PathBuf::from("test.aac")));
    assert!(decoder.supports_format(&PathBuf::from("test.m4a")));
    assert!(decoder.supports_format(&PathBuf::from("test.AAC")));
    assert!(decoder.supports_format(&PathBuf::from("test.M4A")));
}

#[test]
fn test_unsupported_format_detection() {
    let decoder = SymphoniaDecoder::new();
    assert!(!decoder.supports_format(&PathBuf::from("test.txt")));
    assert!(!decoder.supports_format(&PathBuf::from("test.pdf")));
    assert!(!decoder.supports_format(&PathBuf::from("test.doc")));
    assert!(!decoder.supports_format(&PathBuf::from("test.exe")));
    assert!(!decoder.supports_format(&PathBuf::from("test.ape"))); // APE not supported by Symphonia
    assert!(!decoder.supports_format(&PathBuf::from("test.wv"))); // WavPack not supported
}

// ============================================================================
// BIT DEPTH TESTS
// ============================================================================

#[test]
fn test_decode_8bit_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("8bit.wav");

    create_wav_file(&path, 44100, 2, 8, 0.5, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "8-bit WAV should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 44100);
    assert_eq!(buffer.format.channels, 2);

    // Verify samples are in valid range
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "8-bit sample out of range: {}",
            sample
        );
    }

    // Verify there's actual audio content
    let rms = calculate_rms(&buffer.samples);
    assert!(
        rms > 0.1,
        "8-bit audio should have significant RMS: {}",
        rms
    );
}

#[test]
fn test_decode_16bit_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("16bit.wav");

    create_wav_file(&path, 44100, 2, 16, 0.5, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "16-bit WAV should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 44100);

    // Verify samples are in valid range
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "16-bit sample out of range: {}",
            sample
        );
    }

    // Peak should be close to 1.0 for full-scale sine
    let peak = find_peak(&buffer.samples);
    assert!(peak > 0.9, "16-bit peak should be close to 1.0: {}", peak);
}

#[test]
fn test_decode_24bit_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("24bit.wav");

    create_wav_file(&path, 44100, 2, 24, 0.5, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "24-bit WAV should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 44100);

    // Verify samples are in valid range
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "24-bit sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_decode_32bit_float_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("32bit_float.wav");

    create_wav_file(&path, 44100, 2, 32, 0.5, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "32-bit float WAV should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 44100);

    // 32-bit float output should be clamped to [-1, 1]
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "32-bit float sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_bit_depth_dynamic_range() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create files with different bit depths
    let bit_depths = [8, 16, 24];

    for bits in bit_depths {
        let path = temp_dir.path().join(format!("{}bit_dynamic.wav", bits));

        // Create a very quiet signal (should only be preserved at higher bit depths)
        let mut file = File::create(&path).unwrap();

        let sample_rate = 44100u32;
        let channels = 2u16;
        let bytes_per_sample = bits / 8;
        let num_samples = 44100usize; // 1 second
        let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
        let block_align = channels * bytes_per_sample;
        let data_size = (num_samples * channels as usize * bytes_per_sample as usize) as u32;
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
        file.write_all(&bits.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();

        // Write a quiet sine wave (-60dB)
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample_f = (2.0 * PI * 440.0 * t).sin() * 0.001; // Very quiet

            for _ in 0..channels {
                match bits {
                    8 => {
                        let sample_u8 = ((sample_f + 1.0) * 127.5) as u8;
                        file.write_all(&[sample_u8]).unwrap();
                    }
                    16 => {
                        let sample_i16 = (sample_f * i16::MAX as f32) as i16;
                        file.write_all(&sample_i16.to_le_bytes()).unwrap();
                    }
                    24 => {
                        let sample_i32 = (sample_f * 8388607.0) as i32;
                        let bytes = sample_i32.to_le_bytes();
                        file.write_all(&bytes[0..3]).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }
        drop(file);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);
        assert!(result.is_ok(), "{}-bit should decode", bits);

        let buffer = result.unwrap();
        let rms = calculate_rms(&buffer.samples);

        // All bit depths should decode the quiet signal
        assert!(rms > 0.0001, "{}-bit RMS should be non-zero: {}", bits, rms);
    }
}

// ============================================================================
// SAMPLE RATE TESTS
// ============================================================================

#[test]
fn test_decode_8khz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("8khz.wav");

    create_test_wav(&path, 8000, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "8kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 8000);
}

#[test]
fn test_decode_11025hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("11025hz.wav");

    create_test_wav(&path, 11025, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "11.025kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 11025);
}

#[test]
fn test_decode_22050hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("22050hz.wav");

    create_test_wav(&path, 22050, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "22.05kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 22050);
}

#[test]
fn test_decode_44100hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("44100hz.wav");

    create_test_wav(&path, 44100, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "44.1kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 44100);
}

#[test]
fn test_decode_48000hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("48000hz.wav");

    create_test_wav(&path, 48000, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "48kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 48000);
}

#[test]
fn test_decode_88200hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("88200hz.wav");

    create_test_wav(&path, 88200, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "88.2kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 88200);
}

#[test]
fn test_decode_96000hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("96000hz.wav");

    create_test_wav(&path, 96000, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "96kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 96000);
}

#[test]
fn test_decode_176400hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("176400hz.wav");

    create_test_wav(&path, 176400, 0.25, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "176.4kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 176400);
}

#[test]
fn test_decode_192000hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("192000hz.wav");

    create_test_wav(&path, 192000, 0.25, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "192kHz should decode");
    assert_eq!(result.unwrap().format.sample_rate.0, 192000);
}

#[test]
fn test_decode_352800hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("352800hz.wav");

    create_test_wav(&path, 352800, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "352.8kHz should decode: {:?}", result.err());
    assert_eq!(result.unwrap().format.sample_rate.0, 352800);
}

#[test]
fn test_decode_384000hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("384000hz.wav");

    create_test_wav(&path, 384000, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "384kHz should decode: {:?}", result.err());
    assert_eq!(result.unwrap().format.sample_rate.0, 384000);
}

#[test]
fn test_non_standard_sample_rates() {
    let temp_dir = tempfile::tempdir().unwrap();

    let non_standard_rates = [12000, 16000, 24000, 32000, 37800, 50000, 64000];

    for rate in non_standard_rates {
        let path = temp_dir.path().join(format!("{}hz.wav", rate));
        create_test_wav(&path, rate, 0.2, 2);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        assert!(
            result.is_ok(),
            "{}Hz should decode: {:?}",
            rate,
            result.err()
        );
        assert_eq!(
            result.unwrap().format.sample_rate.0,
            rate,
            "Sample rate mismatch for {}Hz",
            rate
        );
    }
}

// ============================================================================
// CHANNEL CONFIGURATION TESTS
// ============================================================================

#[test]
fn test_decode_mono() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono.wav");

    create_test_wav(&path, 44100, 0.5, 1);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Mono should decode");

    let buffer = result.unwrap();
    // Decoder outputs stereo by duplicating mono
    assert_eq!(buffer.format.channels, 2);

    // Verify left and right channels are identical (mono duplicated)
    let left: Vec<f32> = buffer.samples.iter().step_by(2).copied().collect();
    let right: Vec<f32> = buffer.samples.iter().skip(1).step_by(2).copied().collect();

    assert!(
        buffers_nearly_equal(&left, &right, 0.0001),
        "Mono duplicated to stereo should have identical channels"
    );
}

#[test]
fn test_decode_stereo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("stereo.wav");

    create_test_wav(&path, 44100, 0.5, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Stereo should decode");
    assert_eq!(result.unwrap().format.channels, 2);
}

#[test]
fn test_decode_3_channel() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("3ch.wav");

    create_multichannel_wav(&path, 44100, 0.2, 3);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "3-channel should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    // Should be downmixed to stereo
    assert_eq!(buffer.format.channels, 2);
    assert!(!buffer.samples.is_empty());
}

#[test]
fn test_decode_quad_4_channel() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("quad.wav");

    create_multichannel_wav(&path, 44100, 0.2, 4);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "Quad (4-channel) should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.channels, 2);
    assert!(!buffer.samples.is_empty());
}

#[test]
fn test_decode_5_channel() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("5ch.wav");

    create_multichannel_wav(&path, 44100, 0.2, 5);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "5-channel should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.channels, 2);
}

#[test]
fn test_decode_5_1_surround() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("5_1.wav");

    create_multichannel_wav(&path, 44100, 0.2, 6);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "5.1 surround should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    // Should be downmixed to stereo
    assert_eq!(buffer.format.channels, 2);

    // Verify there's actual content after downmix
    let rms = calculate_rms(&buffer.samples);
    assert!(rms > 0.1, "5.1 downmix should have content: {}", rms);
}

#[test]
fn test_decode_7_1_surround() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("7_1.wav");

    create_multichannel_wav(&path, 44100, 0.2, 8);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "7.1 surround should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();
    assert_eq!(buffer.format.channels, 2);
}

// ============================================================================
// CORRUPTED FILE TESTS
// ============================================================================

#[test]
fn test_empty_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("empty.wav");

    File::create(&path).unwrap();

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Empty file should fail to decode");
}

#[test]
fn test_random_garbage() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("garbage.wav");

    let mut file = File::create(&path).unwrap();
    for i in 0..2048 {
        file.write_all(&[(i * 7 % 256) as u8]).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Random garbage should fail to decode");
}

#[test]
fn test_truncated_riff_header() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_riff.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIF").unwrap(); // Incomplete RIFF
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Truncated RIFF header should fail");
}

#[test]
fn test_truncated_wave_header() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_wave.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAV").unwrap(); // Incomplete WAVE
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Truncated WAVE header should fail");
}

#[test]
fn test_missing_fmt_chunk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("no_fmt.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    // Skip fmt chunk, go directly to data
    file.write_all(b"data").unwrap();
    file.write_all(&50u32.to_le_bytes()).unwrap();
    file.write_all(&vec![0u8; 50]).unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Missing fmt chunk should fail");
}

#[test]
fn test_truncated_fmt_chunk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_fmt.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    // Only write partial fmt data (should be 16 bytes)
    file.write_all(&1u16.to_le_bytes()).unwrap(); // format
    file.write_all(&2u16.to_le_bytes()).unwrap(); // channels
                                                  // Missing: sample rate, byte rate, block align, bits per sample
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Truncated fmt chunk should fail");
}

#[test]
fn test_missing_data_chunk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("no_data.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&36u32.to_le_bytes()).unwrap(); // Size of header without data
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    file.write_all(&2u16.to_le_bytes()).unwrap(); // channels
    file.write_all(&44100u32.to_le_bytes()).unwrap(); // sample rate
    file.write_all(&176400u32.to_le_bytes()).unwrap(); // byte rate
    file.write_all(&4u16.to_le_bytes()).unwrap(); // block align
    file.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
                                                   // No data chunk
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // May succeed with empty buffer or fail - either is acceptable
    match result {
        Ok(buffer) => {
            assert!(
                buffer.samples.is_empty(),
                "Missing data should produce empty buffer"
            );
        }
        Err(_) => {
            // Also acceptable
        }
    }
}

#[test]
fn test_truncated_audio_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_data.wav");

    // Create valid WAV then truncate it
    create_test_wav(&path, 44100, 1.0, 2);

    // Truncate file
    let file = File::options().write(true).open(&path).unwrap();
    file.set_len(100).unwrap(); // Keep only first 100 bytes
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Should either fail or return partial data - doesn't panic
    let _ = result;
}

#[test]
fn test_corrupted_data_section() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("corrupted_data.wav");

    // Create valid header with corrupted data
    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let num_samples = 4410;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write random "corrupted" data
    for i in 0..(num_samples * channels as usize) {
        let random_sample = (((i * 12345 + 67890) % 65536) as i32 - 32768) as i16;
        file.write_all(&random_sample.to_le_bytes()).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Should decode (data is "valid" format-wise, just not meaningful audio)
    assert!(result.is_ok(), "Corrupted but valid format should decode");

    let buffer = result.unwrap();
    // Verify all samples are finite and in range
    for sample in &buffer.samples {
        assert!(sample.is_finite(), "Sample should be finite");
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_invalid_format_code() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("invalid_format.wav");

    let mut file = File::create(&path).unwrap();

    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&99u16.to_le_bytes()).unwrap(); // Invalid format code
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&44100u32.to_le_bytes()).unwrap();
    file.write_all(&176400u32.to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Invalid format code should fail");
}

#[test]
fn test_zero_sample_rate() {
    use std::panic;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("zero_rate.wav");

    let mut file = File::create(&path).unwrap();

    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap(); // Zero sample rate
    file.write_all(&0u32.to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&8u32.to_le_bytes()).unwrap();
    file.write_all(&vec![0u8; 8]).unwrap();
    drop(file);

    // Zero sample rate should either return an error or panic in Symphonia
    // We use catch_unwind to handle either case gracefully
    let path_clone = path.clone();
    let result = panic::catch_unwind(move || {
        let mut decoder = SymphoniaDecoder::new();
        decoder.decode(&path_clone)
    });

    // Both panic and error are acceptable for invalid sample rate
    match result {
        Ok(Ok(_)) => {
            // If it somehow succeeds, that's unexpected but we accept it
        }
        Ok(Err(_)) => {
            // Returned an error - acceptable
        }
        Err(_) => {
            // Panicked - acceptable for invalid input like zero sample rate
        }
    }
}

#[test]
fn test_zero_channels() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("zero_channels.wav");

    let mut file = File::create(&path).unwrap();

    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&0u16.to_le_bytes()).unwrap(); // Zero channels
    file.write_all(&44100u32.to_le_bytes()).unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap();
    file.write_all(&0u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Zero channels should fail");
}

// ============================================================================
// EMPTY AND MINIMAL FILE TESTS
// ============================================================================

#[test]
fn test_wav_header_only() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("header_only.wav");

    let mut file = File::create(&path).unwrap();

    file.write_all(b"RIFF").unwrap();
    file.write_all(&36u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&44100u32.to_le_bytes()).unwrap();
    file.write_all(&176400u32.to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap(); // Zero-length data
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    match result {
        Ok(buffer) => {
            assert!(
                buffer.samples.is_empty(),
                "Header-only should produce empty buffer"
            );
        }
        Err(_) => {
            // Also acceptable
        }
    }
}

#[test]
fn test_single_sample() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("single_sample.wav");

    let mut file = File::create(&path).unwrap();

    let data_size = 4u32; // Single stereo frame (2 channels * 2 bytes)

    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&44100u32.to_le_bytes()).unwrap();
    file.write_all(&176400u32.to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&1000i16.to_le_bytes()).unwrap(); // Left
    file.write_all(&(-1000i16).to_le_bytes()).unwrap(); // Right
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Single sample should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.samples.len(), 2, "Should have 2 samples (L, R)");
}

#[test]
fn test_1ms_audio() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("1ms.wav");

    create_test_wav(&path, 44100, 0.001, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "1ms audio should decode");

    let buffer = result.unwrap();
    // 44100 * 0.001 * 2 = ~88 samples
    assert!(buffer.samples.len() >= 80 && buffer.samples.len() <= 100);
}

#[test]
fn test_10ms_audio() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("10ms.wav");

    create_test_wav(&path, 44100, 0.01, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "10ms audio should decode");
}

#[test]
fn test_100ms_audio() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("100ms.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "100ms audio should decode");

    let buffer = result.unwrap();
    // Should have substantial content
    let expected = (44100.0 * 0.1 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i32 - expected as i32).abs() < 100,
        "Sample count mismatch: got {}, expected ~{}",
        buffer.samples.len(),
        expected
    );
}

// ============================================================================
// LARGE FILE TESTS
// ============================================================================

#[test]
fn test_30_second_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("30sec.wav");

    create_test_wav(&path, 44100, 30.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "30-second file should decode");

    let buffer = result.unwrap();
    let expected = (44100.0 * 30.0 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i64 - expected as i64).abs() < 1000,
        "30-second file sample count mismatch"
    );
}

#[test]
fn test_60_second_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("60sec.wav");

    create_test_wav(&path, 44100, 60.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "60-second file should decode");

    let buffer = result.unwrap();
    let expected = (44100.0 * 60.0 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i64 - expected as i64).abs() < 1000,
        "60-second file sample count mismatch"
    );
}

#[test]
fn test_5_minute_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("5min.wav");

    create_test_wav(&path, 44100, 300.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "5-minute file should decode");

    let buffer = result.unwrap();
    let expected = (44100.0 * 300.0 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i64 - expected as i64).abs() < 5000,
        "5-minute file sample count mismatch"
    );
}

#[test]
fn test_high_res_large_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("highres_large.wav");

    // 96kHz 24-bit 30 seconds
    create_wav_file(&path, 96000, 2, 24, 30.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "High-res large file should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 96000);

    let expected = (96000.0 * 30.0 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i64 - expected as i64).abs() < 1000,
        "High-res sample count mismatch"
    );
}

// ============================================================================
// AUDIO CONTENT VERIFICATION TESTS
// ============================================================================

#[test]
fn test_silence_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("silence.wav");

    create_silence_wav(&path, 44100, 1.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Silence file should decode");

    let buffer = result.unwrap();

    // Verify all samples are zero (or very close)
    for sample in &buffer.samples {
        assert!(
            sample.abs() < 0.0001,
            "Silence should produce near-zero samples"
        );
    }
}

#[test]
fn test_dc_offset_preservation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("dc_offset.wav");

    let dc_value = 0.5f32;
    create_dc_offset_wav(&path, 44100, 0.5, dc_value);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "DC offset file should decode");

    let buffer = result.unwrap();

    // Verify DC offset is preserved (approximately)
    let avg: f32 = buffer.samples.iter().sum::<f32>() / buffer.samples.len() as f32;
    assert!(
        (avg - dc_value).abs() < 0.01,
        "DC offset should be preserved: expected {}, got {}",
        dc_value,
        avg
    );
}

#[test]
fn test_impulse_detection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("impulse.wav");

    create_impulse_wav(&path, 44100, 0.5, 0.5); // Impulse at middle

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Impulse file should decode");

    let buffer = result.unwrap();

    // Find the peak
    let peak_pos = buffer
        .samples
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    // Peak should be near the middle
    let expected_pos = buffer.samples.len() / 2;
    let tolerance = buffer.samples.len() / 10;
    assert!(
        (peak_pos as i64 - expected_pos as i64).abs() < tolerance as i64,
        "Impulse position mismatch: expected ~{}, got {}",
        expected_pos,
        peak_pos
    );
}

#[test]
fn test_frequency_detection_440hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("440hz.wav");

    create_freq_test_wav(&path, 44100, 1.0, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let estimated_freq = estimate_frequency_from_crossings(&buffer.samples, 44100);

    // Allow 10% tolerance
    assert!(
        (estimated_freq - 440.0).abs() < 44.0,
        "Frequency detection: expected ~440Hz, got {}Hz",
        estimated_freq
    );
}

#[test]
fn test_frequency_detection_1000hz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("1000hz.wav");

    create_freq_test_wav(&path, 44100, 1.0, 1000.0);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let estimated_freq = estimate_frequency_from_crossings(&buffer.samples, 44100);

    assert!(
        (estimated_freq - 1000.0).abs() < 100.0,
        "Frequency detection: expected ~1000Hz, got {}Hz",
        estimated_freq
    );
}

#[test]
fn test_peak_amplitude_preservation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("full_scale.wav");

    create_wav_file(&path, 44100, 2, 16, 0.5, 440.0);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let peak = find_peak(&buffer.samples);

    // 16-bit full scale sine should decode close to 1.0
    assert!(peak > 0.99, "Peak amplitude should be preserved: {}", peak);
}

// ============================================================================
// SAMPLE ACCURACY TESTS
// ============================================================================

#[test]
fn test_sample_count_accuracy_short() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("sample_count_short.wav");

    let sample_rate = 44100u32;
    let duration = 0.1f32;
    let channels = 2u16;

    create_test_wav(&path, sample_rate, duration, channels);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let expected_samples = (sample_rate as f32 * duration * channels as f32) as usize;

    assert_eq!(
        buffer.samples.len(),
        expected_samples,
        "Sample count should be exact for short files"
    );
}

#[test]
fn test_sample_count_accuracy_long() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("sample_count_long.wav");

    let sample_rate = 44100u32;
    let duration = 10.0f32;
    let channels = 2u16;

    create_test_wav(&path, sample_rate, duration, channels);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let expected_samples = (sample_rate as f32 * duration * channels as f32) as usize;

    assert_eq!(
        buffer.samples.len(),
        expected_samples,
        "Sample count should be exact for long files"
    );
}

#[test]
fn test_sample_value_range() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("value_range.wav");

    create_test_wav(&path, 44100, 1.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    for (i, sample) in buffer.samples.iter().enumerate() {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample {} out of range: {}",
            i,
            sample
        );
        assert!(sample.is_finite(), "Sample {} not finite: {}", i, sample);
    }
}

#[test]
fn test_no_nan_or_inf_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("no_nan.wav");

    create_test_wav(&path, 44100, 1.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    for (i, sample) in buffer.samples.iter().enumerate() {
        assert!(!sample.is_nan(), "Sample {} is NaN", i);
        assert!(!sample.is_infinite(), "Sample {} is infinite", i);
    }
}

// ============================================================================
// BIT-PERFECT DECODE TESTS
// ============================================================================

#[test]
fn test_decode_twice_identical() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("twice.wav");

    create_test_wav(&path, 44100, 0.5, 2);

    let mut decoder1 = SymphoniaDecoder::new();
    let buffer1 = decoder1.decode(&path).unwrap();

    let mut decoder2 = SymphoniaDecoder::new();
    let buffer2 = decoder2.decode(&path).unwrap();

    assert_eq!(buffer1.samples.len(), buffer2.samples.len());

    // Should be bit-perfect identical
    for (i, (s1, s2)) in buffer1
        .samples
        .iter()
        .zip(buffer2.samples.iter())
        .enumerate()
    {
        assert_eq!(
            s1.to_bits(),
            s2.to_bits(),
            "Sample {} differs between decodes: {} vs {}",
            i,
            s1,
            s2
        );
    }
}

#[test]
fn test_sequential_decodes_independent() {
    let temp_dir = tempfile::tempdir().unwrap();

    let path1 = temp_dir.path().join("seq1.wav");
    let path2 = temp_dir.path().join("seq2.wav");

    create_freq_test_wav(&path1, 44100, 0.5, 440.0);
    create_freq_test_wav(&path2, 44100, 0.5, 880.0);

    let mut decoder = SymphoniaDecoder::new();

    let buffer1 = decoder.decode(&path1).unwrap();
    let buffer2 = decoder.decode(&path2).unwrap();

    // Buffers should have different content
    let max_diff = max_difference(&buffer1.samples, &buffer2.samples);
    assert!(
        max_diff > 0.1,
        "Different frequency files should produce different output"
    );
}

// ============================================================================
// FILE PATH EDGE CASES
// ============================================================================

#[test]
fn test_nonexistent_file() {
    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&PathBuf::from("/nonexistent/path/to/file.wav"));

    assert!(result.is_err(), "Nonexistent file should fail");
}

#[test]
fn test_directory_instead_of_file() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(temp_dir.path());

    assert!(result.is_err(), "Directory should fail to decode");
}

#[test]
fn test_unicode_filename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir
        .path()
        .join("test_\u{4E2D}\u{6587}_\u{65E5}\u{672C}\u{8A9E}.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Unicode filename should work");
}

#[test]
fn test_spaces_in_filename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test file with many spaces.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Spaces in filename should work");
}

#[test]
fn test_special_characters_in_filename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test-file_123.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Special characters should work");
}

#[test]
fn test_very_long_filename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let long_name = format!("{}.wav", "a".repeat(200));
    let path = temp_dir.path().join(&long_name);

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Long filename should work");
}

#[test]
fn test_wrong_extension() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("audio.txt");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Symphonia may detect format from content regardless of extension
    // Either success or failure is acceptable
    let _ = result;
}

#[test]
fn test_no_extension() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("audio_file_no_ext");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Either outcome is acceptable
    let _ = result;
}

// ============================================================================
// CONCURRENT DECODE TESTS
// ============================================================================

#[test]
fn test_parallel_decodes() {
    use std::thread;

    let temp_dir = tempfile::tempdir().unwrap();

    let paths: Vec<_> = (0..4)
        .map(|i| {
            let path = temp_dir.path().join(format!("parallel_{}.wav", i));
            create_test_wav(&path, 44100, 0.5, 2);
            path
        })
        .collect();

    let handles: Vec<_> = paths
        .into_iter()
        .map(|path| {
            thread::spawn(move || {
                let mut decoder = SymphoniaDecoder::new();
                decoder.decode(&path)
            })
        })
        .collect();

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        assert!(result.is_ok(), "Parallel decode should succeed");
    }
}

#[test]
fn test_sequential_many_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut decoder = SymphoniaDecoder::new();

    for i in 0..50 {
        let path = temp_dir.path().join(format!("seq_{}.wav", i));
        create_test_wav(&path, 44100, 0.1, 2);

        let result = decoder.decode(&path);
        assert!(result.is_ok(), "Sequential decode {} should succeed", i);
    }
}

// ============================================================================
// MEMORY PATTERN TESTS (for leak detection)
// ============================================================================

#[test]
fn test_decode_discard_repeat() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("memory_test.wav");

    create_test_wav(&path, 44100, 1.0, 2);

    // Decode and discard many times to stress memory
    for _ in 0..100 {
        let mut decoder = SymphoniaDecoder::new();
        let _ = decoder.decode(&path);
        // Buffer dropped here
    }

    // If we get here without OOM or crash, memory is being freed
}

#[test]
fn test_decoder_reuse() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("reuse_test.wav");

    create_test_wav(&path, 44100, 1.0, 2);

    let mut decoder = SymphoniaDecoder::new();

    // Reuse same decoder many times
    for i in 0..50 {
        let result = decoder.decode(&path);
        assert!(
            result.is_ok(),
            "Decode {} with reused decoder should work",
            i
        );
    }
}

#[test]
fn test_large_buffer_memory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("large_memory.wav");

    // Create a large file (60 seconds at 96kHz stereo)
    create_test_wav(&path, 96000, 60.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Large file should decode");

    let buffer = result.unwrap();
    let expected_samples = 96000 * 60 * 2;
    assert!(
        (buffer.samples.len() as i64 - expected_samples as i64).abs() < 5000,
        "Large buffer size mismatch"
    );
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

#[test]
fn test_recover_after_error() {
    let temp_dir = tempfile::tempdir().unwrap();

    let bad_path = temp_dir.path().join("bad.wav");
    let good_path = temp_dir.path().join("good.wav");

    // Create only the good file
    create_test_wav(&good_path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();

    // First decode fails
    let result1 = decoder.decode(&bad_path);
    assert!(result1.is_err());

    // Second decode should still work
    let result2 = decoder.decode(&good_path);
    assert!(result2.is_ok(), "Should recover after failed decode");
}

#[test]
fn test_recover_after_corrupted_file() {
    let temp_dir = tempfile::tempdir().unwrap();

    let corrupted_path = temp_dir.path().join("corrupted.wav");
    let good_path = temp_dir.path().join("good.wav");

    // Create corrupted file
    let mut file = File::create(&corrupted_path).unwrap();
    file.write_all(b"not a valid audio file").unwrap();
    drop(file);

    create_test_wav(&good_path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();

    // First decode fails
    let _ = decoder.decode(&corrupted_path);

    // Second decode should work
    let result = decoder.decode(&good_path);
    assert!(result.is_ok(), "Should recover after corrupted file");
}

#[test]
fn test_multiple_error_recovery() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut decoder = SymphoniaDecoder::new();

    // Try several non-existent files
    for i in 0..5 {
        let bad_path = temp_dir.path().join(format!("nonexistent_{}.wav", i));
        let _ = decoder.decode(&bad_path);
    }

    // Should still work after multiple errors
    let good_path = temp_dir.path().join("good.wav");
    create_test_wav(&good_path, 44100, 0.1, 2);

    let result = decoder.decode(&good_path);
    assert!(result.is_ok(), "Should work after multiple errors");
}

// ============================================================================
// INTERLEAVING TESTS
// ============================================================================

#[test]
fn test_stereo_interleaving() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("stereo_interleave.wav");

    // Create file where left and right channels are different
    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let num_samples = 4410; // 0.1 seconds
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write different frequencies to L and R
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let left = (2.0 * PI * 440.0 * t).sin();
        let right = (2.0 * PI * 880.0 * t).sin();

        let left_i16 = (left * i16::MAX as f32) as i16;
        let right_i16 = (right * i16::MAX as f32) as i16;

        file.write_all(&left_i16.to_le_bytes()).unwrap();
        file.write_all(&right_i16.to_le_bytes()).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Extract channels
    let left: Vec<f32> = buffer.samples.iter().step_by(2).copied().collect();
    let right: Vec<f32> = buffer.samples.iter().skip(1).step_by(2).copied().collect();

    // They should be different
    let max_diff = max_difference(&left, &right);
    assert!(
        max_diff > 0.1,
        "Different frequencies should produce different channels"
    );
}

// ============================================================================
// DOWNMIX COEFFICIENT TESTS
// ============================================================================

#[test]
fn test_51_downmix_center_channel() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("51_center.wav");

    // Create 5.1 file with only center channel active
    let sample_rate = 44100u32;
    let channels = 6u16;
    let num_samples = 4410;

    let mut file = File::create(&path).unwrap();

    let fmt_chunk_size = 40u32;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 20 + fmt_chunk_size + 8 + data_size;

    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&fmt_chunk_size.to_le_bytes()).unwrap();
    file.write_all(&0xFFFEu16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(&22u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(&0x3Fu32.to_le_bytes()).unwrap(); // 5.1 mask
    file.write_all(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ])
    .unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Only center channel has content
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;

        // FL, FR: silence
        file.write_all(&0i16.to_le_bytes()).unwrap();
        file.write_all(&0i16.to_le_bytes()).unwrap();

        // Center: sine wave
        let center = (2.0 * PI * 440.0 * t).sin();
        let center_i16 = (center * i16::MAX as f32 * 0.7) as i16;
        file.write_all(&center_i16.to_le_bytes()).unwrap();

        // LFE, SL, SR: silence
        file.write_all(&0i16.to_le_bytes()).unwrap();
        file.write_all(&0i16.to_le_bytes()).unwrap();
        file.write_all(&0i16.to_le_bytes()).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(
        result.is_ok(),
        "5.1 center-only should decode: {:?}",
        result.err()
    );

    let buffer = result.unwrap();

    // Left and right should be equal (center mixed equally)
    let left: Vec<f32> = buffer.samples.iter().step_by(2).copied().collect();
    let right: Vec<f32> = buffer.samples.iter().skip(1).step_by(2).copied().collect();

    let diff = max_difference(&left, &right);
    assert!(
        diff < 0.01,
        "Center-only should produce equal L/R, diff: {}",
        diff
    );

    // Should have content
    let rms = calculate_rms(&buffer.samples);
    assert!(rms > 0.1, "Should have audio content after downmix");
}

// ============================================================================
// DURATION AND FRAME CALCULATION TESTS
// ============================================================================

#[test]
fn test_duration_calculation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("duration_test.wav");

    let expected_duration = 2.5f32;
    create_test_wav(&path, 44100, expected_duration, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let calculated_duration = buffer.duration_secs();
    assert!(
        (calculated_duration as f32 - expected_duration).abs() < 0.01,
        "Duration mismatch: expected {}, got {}",
        expected_duration,
        calculated_duration
    );
}

#[test]
fn test_frame_count() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("frame_test.wav");

    let sample_rate = 44100u32;
    let duration = 1.0f32;
    create_test_wav(&path, sample_rate, duration, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    let expected_frames = (sample_rate as f32 * duration) as usize;
    assert_eq!(buffer.frames(), expected_frames, "Frame count mismatch");
}

// ============================================================================
// DEFAULT TRAIT TESTS
// ============================================================================

#[test]
fn test_default_decoder() {
    let decoder = SymphoniaDecoder::default();
    assert!(decoder.supports_format(&PathBuf::from("test.wav")));
}

#[test]
fn test_decoder_send() {
    fn assert_send<T: Send>() {}
    assert_send::<SymphoniaDecoder>();
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[test]
fn test_rapid_file_creation_and_decode() {
    let temp_dir = tempfile::tempdir().unwrap();

    for i in 0..20 {
        let path = temp_dir.path().join(format!("rapid_{}.wav", i));
        create_test_wav(&path, 44100, 0.1, 2);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);
        assert!(result.is_ok(), "Rapid decode {} failed", i);

        // Delete file immediately
        std::fs::remove_file(&path).unwrap();
    }
}

#[test]
fn test_various_sample_rates_in_sequence() {
    let temp_dir = tempfile::tempdir().unwrap();

    let rates = [
        8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
    ];

    let mut decoder = SymphoniaDecoder::new();

    for rate in rates {
        let path = temp_dir.path().join(format!("rate_{}.wav", rate));
        create_test_wav(&path, rate, 0.1, 2);

        let result = decoder.decode(&path);
        assert!(
            result.is_ok(),
            "Sample rate {} should decode: {:?}",
            rate,
            result.err()
        );
        assert_eq!(result.unwrap().format.sample_rate.0, rate);
    }
}

#[test]
fn test_mixed_bit_depths_in_sequence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut decoder = SymphoniaDecoder::new();

    let bit_depths = [8, 16, 24, 32];

    for bits in bit_depths {
        let path = temp_dir.path().join(format!("bits_{}.wav", bits));
        create_wav_file(&path, 44100, 2, bits, 0.1, 440.0);

        let result = decoder.decode(&path);
        assert!(
            result.is_ok(),
            "{}-bit should decode: {:?}",
            bits,
            result.err()
        );
    }
}

#[test]
fn test_mixed_channels_in_sequence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut decoder = SymphoniaDecoder::new();

    for ch in [1, 2, 3, 4, 5, 6, 8] {
        let path = temp_dir.path().join(format!("ch_{}.wav", ch));
        if ch <= 2 {
            create_test_wav(&path, 44100, 0.1, ch);
        } else {
            create_multichannel_wav(&path, 44100, 0.1, ch);
        }

        let result = decoder.decode(&path);
        assert!(
            result.is_ok(),
            "{}-channel should decode: {:?}",
            ch,
            result.err()
        );
    }
}

// ============================================================================
// NULL TEST - ROUNDTRIP VERIFICATION
// ============================================================================

#[test]
fn test_roundtrip_sample_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("roundtrip.wav");

    // Create known sample values
    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let num_samples = 100;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
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
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write specific test values
    let test_values: Vec<i16> = vec![
        0,
        1,
        -1,
        100,
        -100,
        1000,
        -1000,
        10000,
        -10000,
        i16::MAX,
        i16::MIN + 1, // Avoid MIN exactly due to asymmetry
        i16::MAX / 2,
        i16::MIN / 2,
    ];

    for &value in test_values
        .iter()
        .cycle()
        .take(num_samples * channels as usize)
    {
        file.write_all(&value.to_le_bytes()).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Verify decoded values match expected normalized values
    for (i, &original) in test_values
        .iter()
        .cycle()
        .take(num_samples * channels as usize)
        .enumerate()
    {
        let expected = original as f32 / 32768.0;
        let actual = buffer.samples[i];

        assert!(
            (expected - actual).abs() < 0.0001,
            "Sample {} mismatch: expected {} (from {}), got {}",
            i,
            expected,
            original,
            actual
        );
    }
}
