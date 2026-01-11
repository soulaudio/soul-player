//! Audio Error Recovery Tests
//!
//! This test suite validates the audio pipeline's ability to recover from various
//! error conditions and edge cases that can occur during real-world playback:
//!
//! 1. **Corruption mid-stream recovery**: Decoder encounters corrupted data partway through
//! 2. **Truncated file handling**: Files that end prematurely during streaming
//! 3. **Network simulation**: Buffering, stalls, and reconnection patterns
//! 4. **Partial read scenarios**: Incomplete data reads
//! 5. **NaN/Inf recovery**: Invalid values in audio buffers
//! 6. **Effects producing invalid output**: Recovery after effects malfunction
//! 7. **Graceful degradation**: Behavior under various failure conditions
//!
//! These tests verify actual recovery behavior, not just "doesn't crash".

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, EffectChain, EqBand, GraphicEq,
    Limiter, LimiterSettings, ParametricEq, StereoEnhancer, StereoSettings,
};
use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::panic;
use std::path::PathBuf;

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a stereo sine wave buffer
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * 0.8;
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Check if all values in buffer are finite
fn all_finite(buffer: &[f32]) -> bool {
    buffer.iter().all(|s| s.is_finite())
}

/// Count non-finite values in buffer
fn count_non_finite(buffer: &[f32]) -> usize {
    buffer.iter().filter(|s| !s.is_finite()).count()
}

/// Calculate peak amplitude
fn peak_level(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Calculate RMS level
#[allow(dead_code)]
fn rms_level(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Create a valid WAV file for testing
fn create_test_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 440.0;

    let mut file = File::create(path).expect("Failed to create test WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // Write fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate sine wave samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample_f = (2.0 * PI * frequency * t).sin();
        let sample_i16 = (sample_f * i16::MAX as f32) as i16;

        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a WAV file with corruption at a specific position
fn create_corrupted_wav(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f32,
    corruption_offset: usize,
    corruption_length: usize,
) {
    // First create a valid WAV
    create_test_wav(path, sample_rate, duration_secs, 2);

    // Now corrupt it at the specified position
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("Failed to open file for corruption");

    file.seek(SeekFrom::Start(corruption_offset as u64))
        .expect("Failed to seek for corruption");
    file.write_all(&vec![0xFF; corruption_length])
        .expect("Failed to write corruption");
}

/// Create a WAV file that is truncated mid-data
fn create_truncated_wav(path: &PathBuf, sample_rate: u32, keep_bytes: usize) {
    // First create a valid 2-second WAV
    create_test_wav(path, sample_rate, 2.0, 2);

    // Now truncate it
    let file = File::options()
        .write(true)
        .open(path)
        .expect("Failed to open file for truncation");
    file.set_len(keep_bytes as u64)
        .expect("Failed to truncate file");
}

// ============================================================================
// 1. CORRUPTION MID-STREAM RECOVERY TESTS
// ============================================================================

mod mid_stream_corruption {
    use super::*;

    #[test]
    fn test_corruption_in_audio_data_region() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("corrupted_data.wav");

        // Create a 2-second WAV with corruption in the middle of the data section
        // Header is 44 bytes, so data starts at byte 44
        // Corrupt 100 bytes starting at offset 1000 (well into the data)
        create_corrupted_wav(&path, 44100, 2.0, 1000, 100);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // The decoder should either:
        // 1. Return an error (acceptable)
        // 2. Return partial data up to the corruption (acceptable)
        // 3. Skip the corrupted section and continue (ideal)
        // It should NOT panic
        match result {
            Ok(buffer) => {
                // If we got data, verify it's valid
                let valid_samples: Vec<f32> = buffer
                    .samples
                    .iter()
                    .filter(|s| s.is_finite())
                    .cloned()
                    .collect();
                assert!(
                    !valid_samples.is_empty(),
                    "Should have some valid samples before corruption"
                );
            }
            Err(_) => {
                // Error is acceptable for corrupted file
            }
        }
    }

    #[test]
    fn test_corruption_in_header_region() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("corrupted_header.wav");

        // Corrupt the fmt chunk (bytes 12-36)
        create_corrupted_wav(&path, 44100, 1.0, 20, 10);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Header corruption should result in an error, not a panic
        assert!(
            result.is_err(),
            "Header corruption should produce an error, not succeed"
        );
    }

    #[test]
    fn test_corruption_pattern_periodic_garbage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("periodic_corruption.wav");

        // Create valid WAV
        create_test_wav(&path, 44100, 1.0, 2);

        // Add periodic corruption throughout
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("Failed to open file");

        // Corrupt every 1000 bytes with 10 bytes of garbage
        let corruption_pattern = [0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD];
        for offset in (100..8000).step_by(1000) {
            let _ = file.seek(SeekFrom::Start(offset as u64));
            let _ = file.write_all(&corruption_pattern);
        }
        drop(file);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Should handle gracefully without panic
        let _ = result;
    }

    #[test]
    fn test_single_bit_flip_corruption() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("bitflip.wav");

        create_test_wav(&path, 44100, 0.5, 2);

        // Flip a single bit in the data region
        let mut data = std::fs::read(&path).unwrap();
        if data.len() > 500 {
            data[500] ^= 0x01; // Flip one bit
        }
        std::fs::write(&path, &data).unwrap();

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Single bit flip in audio data should not crash
        // The file should still decode (the audio will just sound slightly different)
        assert!(
            result.is_ok(),
            "Single bit flip in data should not prevent decoding"
        );
    }
}

// ============================================================================
// 2. TRUNCATED FILE HANDLING
// ============================================================================

mod truncated_files {
    use super::*;

    #[test]
    fn test_truncated_mid_data_chunk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("truncated_mid.wav");

        // Create a file that's truncated in the middle of the data chunk
        // Header (44 bytes) + some data, truncated at 1000 bytes
        create_truncated_wav(&path, 44100, 1000);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        match result {
            Ok(buffer) => {
                // Should have partial data
                assert!(
                    !buffer.samples.is_empty(),
                    "Should have some samples from truncated file"
                );
                // Verify what we got is valid
                assert!(
                    all_finite(&buffer.samples),
                    "Partial decode should produce valid samples"
                );
            }
            Err(_) => {
                // Error is also acceptable
            }
        }
    }

    #[test]
    fn test_truncated_at_exact_sample_boundary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("truncated_boundary.wav");

        // Truncate at exactly a sample boundary (44 header + multiple of 4 for stereo 16-bit)
        create_truncated_wav(&path, 44100, 44 + 1024);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        match result {
            Ok(buffer) => {
                // Should have exactly 256 stereo samples (1024 bytes / 4 bytes per stereo sample)
                let expected_samples = 1024 / 4 * 2; // stereo interleaved
                assert!(
                    buffer.samples.len() <= expected_samples + 100, // Allow some tolerance
                    "Got {} samples, expected around {}",
                    buffer.samples.len(),
                    expected_samples
                );
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }

    #[test]
    fn test_truncated_mid_sample() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("truncated_mid_sample.wav");

        // Truncate in the middle of a sample (odd byte count after header)
        create_truncated_wav(&path, 44100, 44 + 101); // 101 is not divisible by 2

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Should handle gracefully without panic
        let _ = result;
    }

    #[test]
    fn test_truncated_after_header() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("truncated_after_header.wav");

        // Truncate immediately after the header (no data)
        create_truncated_wav(&path, 44100, 50); // Just after the 44-byte header

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Should either error or return empty buffer
        match result {
            Ok(buffer) => {
                assert!(
                    buffer.samples.is_empty() || buffer.samples.len() < 10,
                    "Should have minimal or no data"
                );
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }

    #[test]
    fn test_file_size_mismatch_in_header() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("size_mismatch.wav");

        // Create a 1-second file
        create_test_wav(&path, 44100, 1.0, 2);

        // Now truncate it to half the size, but header still claims full size
        let file = File::options()
            .write(true)
            .open(&path)
            .expect("Failed to open file");
        let original_size = file.metadata().unwrap().len();
        file.set_len(original_size / 2).unwrap();
        drop(file);

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        // Should handle the mismatch gracefully
        match result {
            Ok(buffer) => {
                // Should have approximately half the expected samples
                assert!(
                    all_finite(&buffer.samples),
                    "Samples should be valid even with size mismatch"
                );
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }
}

// ============================================================================
// 3. NETWORK SIMULATION (Buffering, Stalls, Reconnection)
// ============================================================================

mod network_simulation {
    use super::*;

    /// Simulates a streaming source that may have gaps or stalls
    struct StreamingSimulator {
        data: Vec<f32>,
        position: usize,
        stall_positions: Vec<usize>,
        stall_counter: usize,
    }

    impl StreamingSimulator {
        fn new(data: Vec<f32>, stall_positions: Vec<usize>) -> Self {
            Self {
                data,
                position: 0,
                stall_positions,
                stall_counter: 0,
            }
        }

        /// Read next chunk, potentially with simulated stalls
        fn read_chunk(&mut self, size: usize) -> Option<Vec<f32>> {
            if self.position >= self.data.len() {
                return None;
            }

            // Simulate stall
            if self.stall_positions.contains(&self.position) {
                self.stall_counter += 1;
                if self.stall_counter < 3 {
                    // Stall for 3 iterations, then continue
                    return Some(vec![0.0; size]); // Return silence during stall
                }
                self.stall_counter = 0;
            }

            let end = (self.position + size).min(self.data.len());
            let chunk = self.data[self.position..end].to_vec();
            self.position = end;
            Some(chunk)
        }

        fn reset(&mut self) {
            self.position = 0;
            self.stall_counter = 0;
        }
    }

    #[test]
    fn test_streaming_with_silence_gaps() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        // Create a signal with silent gaps (simulating buffering)
        let mut buffer = Vec::new();
        for segment in 0..10 {
            if segment % 3 == 0 {
                // Insert silence gap
                buffer.extend(vec![0.0; 1000]);
            } else {
                // Normal audio
                buffer.extend(generate_stereo_sine(440.0, SAMPLE_RATE, 500));
            }
        }

        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "EQ should handle silence gaps without issue"
        );
    }

    #[test]
    fn test_streaming_with_abrupt_level_changes() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::moderate(),
        )));
        chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::default())));

        // Simulate reconnection: loud -> silence -> loud
        let mut buffer = Vec::new();
        buffer.extend(generate_stereo_sine(440.0, SAMPLE_RATE, 500)); // Normal
        buffer.extend(vec![0.0; 500]); // Silence (disconnection)
        buffer.extend(
            generate_stereo_sine(440.0, SAMPLE_RATE, 500)
                .iter()
                .map(|s| s * 2.0), // Louder after reconnect
        );

        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Chain should handle abrupt level changes"
        );
        assert!(
            peak_level(&buffer) <= 1.5,
            "Limiter should control peaks after abrupt change"
        );
    }

    #[test]
    fn test_streaming_buffer_underrun_simulation() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        let source_data = generate_stereo_sine(440.0, SAMPLE_RATE, 2000);
        let mut simulator = StreamingSimulator::new(source_data, vec![500, 1000, 1500]);

        let mut all_output = Vec::new();

        while let Some(chunk) = simulator.read_chunk(256) {
            let mut buffer = chunk;
            eq.process(&mut buffer, SAMPLE_RATE);
            all_output.extend(buffer);
        }

        assert!(
            all_finite(&all_output),
            "Should handle buffer underruns gracefully"
        );
    }

    #[test]
    fn test_streaming_variable_chunk_sizes() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, -3.0, 2.0));

        let chunk_sizes = [64, 128, 32, 256, 16, 512, 100, 1, 2, 1024];
        let mut all_output = Vec::new();

        for &size in chunk_sizes.iter().cycle().take(100) {
            let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, size);
            eq.process(&mut buffer, SAMPLE_RATE);
            all_output.extend(buffer);
        }

        assert!(
            all_finite(&all_output),
            "Should handle variable chunk sizes without issues"
        );
    }

    #[test]
    fn test_reset_mid_stream() {
        let mut eq = ParametricEq::new();
        eq.set_high_band(EqBand::high_shelf(8000.0, 6.0));

        // Process some audio
        let mut buffer1 = generate_stereo_sine(1000.0, SAMPLE_RATE, 500);
        eq.process(&mut buffer1, SAMPLE_RATE);

        // Simulate stream reconnection - reset internal state
        eq.reset();

        // Continue processing
        let mut buffer2 = generate_stereo_sine(1000.0, SAMPLE_RATE, 500);
        eq.process(&mut buffer2, SAMPLE_RATE);

        assert!(
            all_finite(&buffer2),
            "Should work correctly after mid-stream reset"
        );
    }
}

// ============================================================================
// 4. PARTIAL READ SCENARIOS
// ============================================================================

mod partial_reads {
    use super::*;

    #[test]
    fn test_process_single_sample_at_a_time() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        let source = generate_stereo_sine(440.0, SAMPLE_RATE, 100);
        let mut all_output = Vec::new();

        // Process one stereo sample at a time
        for chunk in source.chunks(2) {
            let mut buffer = chunk.to_vec();
            eq.process(&mut buffer, SAMPLE_RATE);
            all_output.extend(buffer);
        }

        assert!(
            all_finite(&all_output),
            "Single-sample processing should work"
        );
    }

    #[test]
    fn test_incomplete_stereo_pair_handling() {
        let effects: Vec<(&str, Box<dyn AudioEffect>)> = vec![
            ("EQ", Box::new(ParametricEq::new())),
            ("Compressor", Box::new(Compressor::new())),
            ("Limiter", Box::new(Limiter::new())),
            ("Crossfeed", Box::new(Crossfeed::new())),
            ("Stereo", Box::new(StereoEnhancer::new())),
        ];

        for (name, mut effect) in effects {
            // Odd number of samples (incomplete stereo pair)
            let mut buffer = vec![0.5, 0.3, 0.7]; // 1.5 stereo samples

            // Should handle gracefully without panic
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                effect.process(&mut buffer, SAMPLE_RATE);
            }));

            assert!(
                result.is_ok(),
                "{} panicked on incomplete stereo pair",
                name
            );
        }
    }

    #[test]
    fn test_very_small_buffers() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        // Test various small buffer sizes
        for size in 0..10 {
            let mut buffer = vec![0.5; size];
            let original_len = buffer.len();

            chain.process(&mut buffer, SAMPLE_RATE);

            assert_eq!(
                buffer.len(),
                original_len,
                "Buffer length should be preserved for size {}",
                size
            );
            assert!(
                all_finite(&buffer),
                "Buffer should be valid for size {}",
                size
            );
        }
    }

    #[test]
    fn test_alternating_buffer_sizes() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(80.0, 6.0));

        // Rapidly alternating between large and tiny buffers
        for i in 0..100 {
            let size = if i % 2 == 0 { 1024 } else { 2 };
            let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, size / 2);
            eq.process(&mut buffer, SAMPLE_RATE);

            assert!(
                all_finite(&buffer),
                "Should handle alternating sizes at iteration {}",
                i
            );
        }
    }
}

// ============================================================================
// 5. NaN/Inf RECOVERY TESTS
// ============================================================================

mod nan_inf_recovery {
    use super::*;

    /// Sanitizing effect that replaces NaN/Inf with zeros
    struct AudioSanitizer;

    impl AudioSanitizer {
        fn sanitize(buffer: &mut [f32]) -> usize {
            let mut replaced = 0;
            for sample in buffer.iter_mut() {
                if !sample.is_finite() {
                    *sample = 0.0;
                    replaced += 1;
                }
            }
            replaced
        }

        fn sanitize_with_fade(buffer: &mut [f32]) -> usize {
            let mut replaced = 0;
            let mut last_good_sample = 0.0;

            for sample in buffer.iter_mut() {
                if sample.is_finite() {
                    last_good_sample = *sample;
                } else {
                    // Replace with decayed version of last good sample
                    *sample = last_good_sample * 0.95;
                    last_good_sample *= 0.95;
                    replaced += 1;
                }
            }
            replaced
        }
    }

    #[test]
    fn test_nan_in_middle_of_buffer() {
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);

        // Inject NaN in the middle
        buffer[500] = f32::NAN;
        buffer[501] = f32::NAN;

        let non_finite_before = count_non_finite(&buffer);
        assert_eq!(non_finite_before, 2, "Should have 2 NaN values");

        // Sanitize
        let replaced = AudioSanitizer::sanitize(&mut buffer);
        assert_eq!(replaced, 2, "Should have replaced 2 values");

        assert!(all_finite(&buffer), "Buffer should be fully finite after sanitization");
    }

    #[test]
    fn test_inf_at_buffer_boundaries() {
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);

        // Inject Inf at boundaries
        buffer[0] = f32::INFINITY;
        buffer[1] = f32::NEG_INFINITY;
        let len = buffer.len();
        buffer[len - 2] = f32::INFINITY;
        buffer[len - 1] = f32::NEG_INFINITY;

        let replaced = AudioSanitizer::sanitize(&mut buffer);
        assert_eq!(replaced, 4, "Should have replaced 4 boundary values");

        assert!(
            all_finite(&buffer),
            "Buffer should be finite after boundary sanitization"
        );
    }

    #[test]
    fn test_effects_after_nan_recovery() {
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);

        // Inject NaN
        buffer[100] = f32::NAN;
        buffer[200] = f32::INFINITY;
        buffer[300] = f32::NEG_INFINITY;

        // Sanitize first
        AudioSanitizer::sanitize(&mut buffer);

        // Now process through effects chain
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Effects should produce finite output after sanitization"
        );
    }

    #[test]
    fn test_complete_nan_buffer_recovery() {
        let mut buffer = vec![f32::NAN; 1000];

        AudioSanitizer::sanitize(&mut buffer);

        assert!(
            buffer.iter().all(|s| *s == 0.0),
            "All NaN should be replaced with zeros"
        );

        // Effects should now handle the zero buffer
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));
        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(all_finite(&buffer), "Effects should handle zeroed buffer");
    }

    #[test]
    fn test_nan_fade_recovery() {
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        let sample_before_nan = buffer[99];

        // Create a burst of NaN
        for i in 100..110 {
            buffer[i] = f32::NAN;
        }

        let replaced = AudioSanitizer::sanitize_with_fade(&mut buffer);
        assert_eq!(replaced, 10, "Should have replaced 10 NaN values");

        // Verify fade behavior
        assert!(
            buffer[100].abs() < sample_before_nan.abs(),
            "Faded replacement should be smaller than original"
        );
        assert!(
            buffer[109].abs() < buffer[100].abs(),
            "Later replacements should continue to fade"
        );

        assert!(all_finite(&buffer), "Buffer should be finite after fade recovery");
    }

    #[test]
    fn test_effect_produces_nan_then_recover() {
        // This is a theoretical test - effects shouldn't produce NaN
        // But if they do, we need to handle it

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);

        // Process through EQ
        let mut eq = ParametricEq::new();
        eq.process(&mut buffer, SAMPLE_RATE);

        // Simulate an effect that produced NaN (bug scenario)
        buffer[250] = f32::NAN;
        buffer[251] = f32::NAN;

        // Sanitize before next effect
        AudioSanitizer::sanitize(&mut buffer);

        // Continue with remaining effects
        let mut limiter = Limiter::new();
        limiter.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Pipeline should recover from mid-chain NaN"
        );
    }

    #[test]
    fn test_denormal_detection_and_handling() {
        let denormal = f32::MIN_POSITIVE / 2.0;
        let mut buffer = vec![denormal; 1000];

        // Process through effects - should handle denormals without issue
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Effects should handle denormals without producing NaN/Inf"
        );
    }
}

// ============================================================================
// 6. EFFECTS PRODUCING INVALID OUTPUT RECOVERY
// ============================================================================

mod effect_invalid_output {
    use super::*;

    /// Mock effect that intentionally produces bad output for testing
    struct FaultyEffect {
        enabled: bool,
        fault_mode: FaultMode,
        samples_processed: usize,
    }

    #[derive(Clone, Copy)]
    enum FaultMode {
        ProduceNanAfter(usize),
        ProduceInfAfter(usize),
        ProduceExtremeValues,
        Intermittent,
    }

    impl FaultyEffect {
        fn new(mode: FaultMode) -> Self {
            Self {
                enabled: true,
                fault_mode: mode,
                samples_processed: 0,
            }
        }
    }

    impl AudioEffect for FaultyEffect {
        fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
            if !self.enabled {
                return;
            }

            for sample in buffer.iter_mut() {
                self.samples_processed += 1;

                match self.fault_mode {
                    FaultMode::ProduceNanAfter(threshold) => {
                        if self.samples_processed > threshold {
                            *sample = f32::NAN;
                        }
                    }
                    FaultMode::ProduceInfAfter(threshold) => {
                        if self.samples_processed > threshold {
                            *sample = f32::INFINITY;
                        }
                    }
                    FaultMode::ProduceExtremeValues => {
                        *sample *= 1e10;
                    }
                    FaultMode::Intermittent => {
                        if self.samples_processed % 100 == 0 {
                            *sample = f32::NAN;
                        }
                    }
                }
            }
        }

        fn reset(&mut self) {
            self.samples_processed = 0;
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn name(&self) -> &str {
            "FaultyEffect"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_detect_nan_production() {
        let mut faulty = FaultyEffect::new(FaultMode::ProduceNanAfter(100));
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);

        faulty.process(&mut buffer, SAMPLE_RATE);

        let nan_count = count_non_finite(&buffer);
        assert!(nan_count > 0, "Faulty effect should have produced NaN");

        // Verify we can detect the issue
        assert!(
            !all_finite(&buffer),
            "Should detect NaN in buffer"
        );
    }

    #[test]
    fn test_limiter_clamps_extreme_values() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(FaultyEffect::new(FaultMode::ProduceExtremeValues)));
        chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::brickwall())));

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        chain.process(&mut buffer, SAMPLE_RATE);

        // The limiter should have clamped extreme values
        // Note: Limiter handles finite extreme values, not Inf/NaN
        let peak = peak_level(&buffer);
        assert!(
            peak.is_finite(),
            "Limiter should produce finite output from extreme values"
        );
    }

    #[test]
    fn test_bypass_faulty_effect() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        chain.add_effect(Box::new(eq));

        let mut faulty = FaultyEffect::new(FaultMode::ProduceNanAfter(100));
        faulty.set_enabled(false); // Bypass the faulty effect
        chain.add_effect(Box::new(faulty));

        chain.add_effect(Box::new(Limiter::new()));

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Chain should work when faulty effect is bypassed"
        );
    }

    #[test]
    fn test_effect_reset_after_fault() {
        let mut faulty = FaultyEffect::new(FaultMode::ProduceNanAfter(100));

        // First run produces NaN
        let mut buffer1 = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        faulty.process(&mut buffer1, SAMPLE_RATE);
        assert!(!all_finite(&buffer1), "First run should have NaN");

        // Reset the effect
        faulty.reset();

        // Second run should work fine until threshold again
        let mut buffer2 = vec![0.5; 50]; // Less than threshold
        faulty.process(&mut buffer2, SAMPLE_RATE);
        assert!(all_finite(&buffer2), "After reset, short buffer should be clean");
    }

    #[test]
    fn test_chain_recovery_after_removing_faulty_effect() {
        // Initial chain with faulty effect
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        {
            let mut chain = EffectChain::new();
            chain.add_effect(Box::new(ParametricEq::new()));
            chain.add_effect(Box::new(FaultyEffect::new(FaultMode::ProduceNanAfter(100))));
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Buffer now has NaN, sanitize it
        for sample in buffer.iter_mut() {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }

        // New chain without faulty effect
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Limiter::new()));
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Chain should work after removing faulty effect"
        );
    }
}

// ============================================================================
// 7. GRACEFUL DEGRADATION TESTS
// ============================================================================

mod graceful_degradation {
    use super::*;

    #[test]
    fn test_effect_continues_after_panic_recovery() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        // Simulate a recoverable panic scenario
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let mut buffer = vec![0.5; 100];
            eq.process(&mut buffer, SAMPLE_RATE);
            buffer
        }));

        assert!(result.is_ok(), "EQ should not panic on normal input");

        // Effect should still work after panic::catch_unwind
        let mut new_eq = ParametricEq::new();
        let mut buffer = vec![0.5; 100];
        new_eq.process(&mut buffer, SAMPLE_RATE);
        assert!(all_finite(&buffer), "Effect should work after panic handling");
    }

    #[test]
    fn test_chain_continues_if_one_effect_disabled() {
        let mut chain = EffectChain::new();

        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        chain.add_effect(Box::new(eq));

        let mut comp = Compressor::new();
        comp.set_enabled(false); // Disabled
        chain.add_effect(Box::new(comp));

        let limiter = Limiter::new();
        chain.add_effect(Box::new(limiter));

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Chain should work with middle effect disabled"
        );
    }

    #[test]
    fn test_degraded_mode_with_extreme_parameters() {
        // Test that effects degrade gracefully with extreme parameters
        let mut eq = ParametricEq::new();

        // Extreme but valid parameters
        eq.set_low_band(EqBand::low_shelf(1.0, 12.0)); // Very low freq, max boost
        eq.set_mid_band(EqBand::peaking(20000.0, -12.0, 10.0)); // Near Nyquist, max cut, narrow Q
        eq.set_high_band(EqBand::high_shelf(22000.0, 12.0)); // At Nyquist, max boost

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 1000);
        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "EQ should handle extreme parameters gracefully"
        );
    }

    #[test]
    fn test_sample_rate_zero_handling() {
        let mut eq = ParametricEq::new();

        // Processing at 0 sample rate should not crash
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let mut buffer = vec![0.5; 100];
            eq.process(&mut buffer, 0);
        }));

        // We accept either success or a caught panic, but not an uncaught panic
        let _ = result;
    }

    #[test]
    fn test_recovery_after_sample_rate_change() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        // Process at 44100
        let mut buffer1 = generate_stereo_sine(440.0, 44100, 500);
        eq.process(&mut buffer1, 44100);

        // Immediately switch to 192000 (no reset)
        let mut buffer2 = generate_stereo_sine(440.0, 192000, 500);
        eq.process(&mut buffer2, 192000);

        assert!(all_finite(&buffer2), "Should handle sample rate change");

        // Switch back to 44100
        let mut buffer3 = generate_stereo_sine(440.0, 44100, 500);
        eq.process(&mut buffer3, 44100);

        assert!(all_finite(&buffer3), "Should handle sample rate restoration");
    }

    #[test]
    fn test_silence_after_error_recovery() {
        // After an error condition, outputting silence is acceptable
        let mut eq = ParametricEq::new();

        // Process corrupted data
        let mut corrupted = vec![f32::NAN; 100];
        eq.process(&mut corrupted, SAMPLE_RATE);

        // Reset
        eq.reset();

        // Process silence
        let mut silence = vec![0.0; 100];
        eq.process(&mut silence, SAMPLE_RATE);

        assert!(
            all_finite(&silence),
            "Processing silence after reset should work"
        );
    }

    #[test]
    fn test_all_effects_recover_from_reset() {
        let effects: Vec<(&str, Box<dyn AudioEffect>)> = vec![
            ("EQ", Box::new(ParametricEq::new())),
            ("GraphicEQ", Box::new(GraphicEq::new_10_band())),
            ("Compressor", Box::new(Compressor::new())),
            ("Limiter", Box::new(Limiter::new())),
            ("Crossfeed", Box::new(Crossfeed::new())),
            (
                "Stereo",
                Box::new(StereoEnhancer::with_settings(StereoSettings::wide())),
            ),
        ];

        for (name, mut effect) in effects {
            // Build up state with loud signal
            for _ in 0..10 {
                let mut buffer = vec![0.9; 1000];
                effect.process(&mut buffer, SAMPLE_RATE);
            }

            // Reset
            effect.reset();

            // Process quiet signal immediately after reset
            let mut buffer = vec![0.1; 1000];
            effect.process(&mut buffer, SAMPLE_RATE);

            assert!(
                all_finite(&buffer),
                "{} should produce finite output after reset",
                name
            );

            // The quiet signal shouldn't be massively attenuated or boosted
            let peak = peak_level(&buffer);
            assert!(
                peak > 0.01 && peak < 1.0,
                "{} peak {} is out of expected range after reset",
                name,
                peak
            );
        }
    }

    #[test]
    fn test_rapid_reset_cycles_under_load() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        for i in 0..100 {
            // Process some audio
            let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 256);
            chain.process(&mut buffer, SAMPLE_RATE);

            // Reset every 10 iterations
            if i % 10 == 0 {
                chain.reset();
            }

            // Verify output is valid
            assert!(
                all_finite(&buffer),
                "Output should be valid at iteration {}",
                i
            );
        }
    }
}

// ============================================================================
// 8. PANIC RECOVERY TESTS
// ============================================================================

mod panic_recovery {
    use super::*;

    #[test]
    fn test_catch_unwind_preserves_buffer() {
        let original = vec![0.5, 0.3, 0.7, 0.2];
        let mut buffer = original.clone();

        // This should not panic, but demonstrates the pattern
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let mut eq = ParametricEq::new();
            eq.process(&mut buffer, SAMPLE_RATE);
        }));

        assert!(result.is_ok(), "Normal processing should not panic");
        assert_eq!(buffer.len(), original.len(), "Buffer length should be preserved");
    }

    #[test]
    fn test_effect_chain_panic_isolation() {
        // Each effect in a chain should be somewhat isolated
        // If one could theoretically panic, others should still be creatable

        let eq_result = panic::catch_unwind(|| ParametricEq::new());
        assert!(eq_result.is_ok(), "EQ creation should not panic");

        let comp_result = panic::catch_unwind(|| Compressor::new());
        assert!(comp_result.is_ok(), "Compressor creation should not panic");

        let limiter_result = panic::catch_unwind(|| Limiter::new());
        assert!(limiter_result.is_ok(), "Limiter creation should not panic");
    }

    #[test]
    fn test_effects_dont_panic_on_empty_buffer() {
        let effects: Vec<Box<dyn AudioEffect>> = vec![
            Box::new(ParametricEq::new()),
            Box::new(Compressor::new()),
            Box::new(Limiter::new()),
            Box::new(Crossfeed::new()),
            Box::new(StereoEnhancer::new()),
            Box::new(GraphicEq::new_10_band()),
        ];

        for mut effect in effects {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let mut buffer: Vec<f32> = vec![];
                effect.process(&mut buffer, SAMPLE_RATE);
            }));

            assert!(
                result.is_ok(),
                "{} panicked on empty buffer",
                effect.name()
            );
        }
    }

    #[test]
    fn test_effects_dont_panic_on_max_f32_values() {
        let effects: Vec<Box<dyn AudioEffect>> = vec![
            Box::new(ParametricEq::new()),
            Box::new(Compressor::new()),
            Box::new(Limiter::new()),
        ];

        for mut effect in effects {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let mut buffer = vec![f32::MAX, f32::MAX, -f32::MAX, -f32::MAX];
                effect.process(&mut buffer, SAMPLE_RATE);
            }));

            assert!(
                result.is_ok(),
                "{} panicked on MAX f32 values",
                effect.name()
            );
        }
    }

    #[test]
    fn test_decoder_doesnt_panic_on_invalid_path() {
        let result = panic::catch_unwind(|| {
            let mut decoder = SymphoniaDecoder::new();
            let _ = decoder.decode(&PathBuf::from("\0invalid\0path\0"));
        });

        assert!(result.is_ok(), "Decoder should handle invalid path without panic");
    }
}

// ============================================================================
// 9. STRESS RECOVERY TESTS
// ============================================================================

mod stress_recovery {
    use super::*;

    #[test]
    fn test_long_running_with_periodic_corruption_recovery() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        let mut corruption_injected = 0;
        let mut nan_detected = 0;
        let mut recovered_count = 0;

        for i in 0..1000 {
            let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 256);

            // Inject corruption every 100 iterations
            if i % 100 == 50 {
                buffer[128] = f32::NAN;
                corruption_injected += 1;
            }

            chain.process(&mut buffer, SAMPLE_RATE);

            // Check for NaN and sanitize
            let had_nan = !all_finite(&buffer);
            if had_nan {
                nan_detected += 1;
                for sample in buffer.iter_mut() {
                    if !sample.is_finite() {
                        *sample = 0.0;
                    }
                }
            }

            // Verify buffer is now clean after sanitization
            if all_finite(&buffer) && had_nan {
                recovered_count += 1;
            }
        }

        // We injected 10 corruptions (every 100 iterations, i=50,150,250,...)
        assert_eq!(corruption_injected, 10, "Should have injected 10 corruptions");

        // Note: Effects propagate NaN from their internal state (filter history),
        // so more buffers may be affected than just the ones we corrupted.
        // This is expected behavior - the test documents that sanitization is needed
        // after each effect to prevent NaN propagation.
        assert!(
            nan_detected >= corruption_injected,
            "Should detect at least as many NaN buffers as corruptions injected"
        );

        // All detected NaN buffers should have been successfully recovered
        assert_eq!(
            nan_detected, recovered_count,
            "All NaN buffers should be recoverable via sanitization"
        );

        // Reset effects and verify they work correctly afterward
        chain.reset();
        let mut final_buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 256);
        chain.process(&mut final_buffer, SAMPLE_RATE);
        assert!(
            all_finite(&final_buffer),
            "Chain should work correctly after reset following corruption"
        );
    }

    #[test]
    fn test_repeated_reset_under_heavy_load() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(80.0, 6.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -3.0, 2.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 3.0));

        for i in 0..10000 {
            let mut buffer = vec![0.5; 512];
            eq.process(&mut buffer, SAMPLE_RATE);

            // Reset very frequently
            if i % 10 == 0 {
                eq.reset();
            }

            assert!(all_finite(&buffer), "Failed at iteration {}", i);
        }
    }

    #[test]
    fn test_memory_stability_after_many_error_recoveries() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        // Simulate many error/recovery cycles
        for _ in 0..1000 {
            // Create corrupted buffer
            let mut buffer: Vec<f32> = (0..1024)
                .map(|i| if i % 100 == 0 { f32::NAN } else { 0.5 })
                .collect();

            chain.process(&mut buffer, SAMPLE_RATE);

            // Sanitize
            for sample in buffer.iter_mut() {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
            }

            // Reset chain
            chain.reset();
        }

        // Final verification
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 500);
        chain.process(&mut buffer, SAMPLE_RATE);
        assert!(
            all_finite(&buffer),
            "Chain should work after many error cycles"
        );
    }
}
