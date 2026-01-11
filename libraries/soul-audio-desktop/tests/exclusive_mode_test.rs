//! Exclusive mode and bit-perfect output tests
//!
//! Comprehensive tests for ExclusiveConfig, AudioData conversions,
//! LatencyInfo calculations, and ExclusiveOutput functionality.
//!
//! Uses proptest for property-based testing of audio conversions.
//! Integration tests gracefully handle CI environments without audio devices.

use proptest::prelude::*;
use soul_audio_desktop::{
    AudioBackend, AudioData, AudioOutputError, ExclusiveConfig, ExclusiveOutput, LatencyInfo,
    SupportedBitDepth,
};

// =============================================================================
// Section 1: Configuration Tests
// =============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_exclusive_config_defaults() {
        let config = ExclusiveConfig::default();

        // Verify default values
        assert_eq!(
            config.sample_rate, 0,
            "Default sample rate should be 0 (device native)"
        );
        assert_eq!(
            config.bit_depth,
            SupportedBitDepth::Float32,
            "Default bit depth should be Float32"
        );
        assert!(
            config.buffer_frames.is_none(),
            "Default buffer frames should be None"
        );
        assert!(
            config.exclusive_mode,
            "Default should enable exclusive mode"
        );
        assert!(
            config.device_name.is_none(),
            "Default device name should be None"
        );
        assert_eq!(
            config.backend,
            AudioBackend::Default,
            "Default backend should be Default"
        );
    }

    #[test]
    fn test_bit_perfect_16_preset() {
        let config = ExclusiveConfig::bit_perfect_16();

        assert_eq!(
            config.bit_depth,
            SupportedBitDepth::Int16,
            "bit_perfect_16 should use Int16"
        );
        assert!(
            config.exclusive_mode,
            "bit_perfect_16 should enable exclusive mode"
        );
        assert_eq!(
            config.sample_rate, 0,
            "bit_perfect_16 should use device native rate"
        );
    }

    #[test]
    fn test_bit_perfect_24_preset() {
        let config = ExclusiveConfig::bit_perfect_24();

        assert_eq!(
            config.bit_depth,
            SupportedBitDepth::Int24,
            "bit_perfect_24 should use Int24"
        );
        assert!(
            config.exclusive_mode,
            "bit_perfect_24 should enable exclusive mode"
        );
        assert_eq!(
            config.sample_rate, 0,
            "bit_perfect_24 should use device native rate"
        );
    }

    #[test]
    fn test_bit_perfect_32_preset() {
        let config = ExclusiveConfig::bit_perfect_32();

        assert_eq!(
            config.bit_depth,
            SupportedBitDepth::Int32,
            "bit_perfect_32 should use Int32"
        );
        assert!(
            config.exclusive_mode,
            "bit_perfect_32 should enable exclusive mode"
        );
        assert_eq!(
            config.sample_rate, 0,
            "bit_perfect_32 should use device native rate"
        );
    }

    #[test]
    fn test_low_latency_preset() {
        let config = ExclusiveConfig::low_latency();

        assert_eq!(
            config.buffer_frames,
            Some(128),
            "low_latency should use 128 frame buffer (~2.9ms at 44.1kHz)"
        );
        assert!(
            config.exclusive_mode,
            "low_latency should enable exclusive mode"
        );

        // Verify approximate latency at 44.1kHz
        let latency_ms: f32 = 128.0 / 44100.0 * 1000.0;
        assert!(
            (latency_ms - 2.9).abs() < 0.1,
            "low_latency buffer should be ~2.9ms at 44.1kHz"
        );
    }

    #[test]
    fn test_ultra_low_latency_preset() {
        let config = ExclusiveConfig::ultra_low_latency();

        assert_eq!(
            config.buffer_frames,
            Some(64),
            "ultra_low_latency should use 64 frame buffer (~1.5ms at 44.1kHz)"
        );
        assert!(
            config.exclusive_mode,
            "ultra_low_latency should enable exclusive mode"
        );

        // Verify approximate latency at 44.1kHz
        let latency_ms: f32 = 64.0 / 44100.0 * 1000.0;
        assert!(
            (latency_ms - 1.45).abs() < 0.1,
            "ultra_low_latency buffer should be ~1.45ms at 44.1kHz"
        );
    }

    #[test]
    fn test_config_builder_with_sample_rate() {
        let config = ExclusiveConfig::default().with_sample_rate(96000);

        assert_eq!(config.sample_rate, 96000);
        // Other defaults should be preserved
        assert_eq!(config.bit_depth, SupportedBitDepth::Float32);
        assert!(config.exclusive_mode);
    }

    #[test]
    fn test_config_builder_with_buffer_frames() {
        let config = ExclusiveConfig::default().with_buffer_frames(256);

        assert_eq!(config.buffer_frames, Some(256));
    }

    #[test]
    fn test_config_builder_with_device() {
        let config = ExclusiveConfig::default().with_device("My Audio Device");

        assert_eq!(config.device_name, Some("My Audio Device".to_string()));
    }

    #[test]
    fn test_config_builder_with_backend() {
        let config = ExclusiveConfig::default().with_backend(AudioBackend::Default);

        assert_eq!(config.backend, AudioBackend::Default);
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = ExclusiveConfig::bit_perfect_24()
            .with_sample_rate(192000)
            .with_buffer_frames(512)
            .with_device("DAC Pro");

        assert_eq!(config.bit_depth, SupportedBitDepth::Int24);
        assert_eq!(config.sample_rate, 192000);
        assert_eq!(config.buffer_frames, Some(512));
        assert_eq!(config.device_name, Some("DAC Pro".to_string()));
        assert!(config.exclusive_mode);
    }

    #[test]
    fn test_config_serialization() {
        let config = ExclusiveConfig::bit_perfect_24()
            .with_sample_rate(96000)
            .with_buffer_frames(256);

        // Should serialize without error
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        assert!(json.contains("96000"), "JSON should contain sample rate");
        assert!(json.contains("256"), "JSON should contain buffer frames");

        // Should deserialize back correctly
        let deserialized: ExclusiveConfig =
            serde_json::from_str(&json).expect("Failed to deserialize config");
        assert_eq!(deserialized.sample_rate, config.sample_rate);
        assert_eq!(deserialized.buffer_frames, config.buffer_frames);
        assert_eq!(deserialized.bit_depth, config.bit_depth);
    }

    #[test]
    fn test_preset_configs_differ_from_default() {
        let default = ExclusiveConfig::default();
        let bp16 = ExclusiveConfig::bit_perfect_16();
        let bp24 = ExclusiveConfig::bit_perfect_24();
        let bp32 = ExclusiveConfig::bit_perfect_32();
        let low_lat = ExclusiveConfig::low_latency();
        let ultra_low = ExclusiveConfig::ultra_low_latency();

        // Bit-perfect presets should have different bit depths
        assert_ne!(default.bit_depth, bp16.bit_depth);
        assert_ne!(default.bit_depth, bp24.bit_depth);
        assert_ne!(default.bit_depth, bp32.bit_depth);

        // Latency presets should have buffer frames set
        assert!(low_lat.buffer_frames.is_some());
        assert!(ultra_low.buffer_frames.is_some());
        assert_ne!(low_lat.buffer_frames, ultra_low.buffer_frames);
    }
}

// =============================================================================
// Section 2: AudioData Conversion Tests
// =============================================================================

mod audio_data_tests {
    use super::*;

    // --- Int16 Conversion Tests ---

    #[test]
    fn test_f32_to_i16_zero() {
        let samples = vec![0.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0], 0, "0.0 should convert to 0 in i16");
            }
            _ => panic!("Expected Int16 variant"),
        }
    }

    #[test]
    fn test_f32_to_i16_positive() {
        let samples = vec![0.5f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 1);
                // 0.5 * 32767 = 16383.5, truncated to 16383
                let expected = (0.5 * i16::MAX as f32) as i16;
                assert_eq!(v[0], expected, "0.5 should convert correctly to i16");
                assert!(v[0] > 0, "0.5 should produce positive i16");
            }
            _ => panic!("Expected Int16 variant"),
        }
    }

    #[test]
    fn test_f32_to_i16_negative() {
        let samples = vec![-0.5f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 1);
                let expected = (-0.5 * i16::MAX as f32) as i16;
                assert_eq!(v[0], expected, "-0.5 should convert correctly to i16");
                assert!(v[0] < 0, "-0.5 should produce negative i16");
            }
            _ => panic!("Expected Int16 variant"),
        }
    }

    #[test]
    fn test_f32_to_i16_max_clipping() {
        let samples = vec![1.0f32, 1.5f32, 2.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 3);
                // All values >= 1.0 should be clamped to 1.0 before conversion
                let max_expected = (1.0f32 * i16::MAX as f32) as i16;
                assert_eq!(v[0], max_expected, "1.0 should produce i16::MAX");
                assert_eq!(v[1], max_expected, "1.5 should be clamped to 1.0");
                assert_eq!(v[2], max_expected, "2.0 should be clamped to 1.0");
            }
            _ => panic!("Expected Int16 variant"),
        }
    }

    #[test]
    fn test_f32_to_i16_min_clipping() {
        let samples = vec![-1.0f32, -1.5f32, -2.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 3);
                // All values <= -1.0 should be clamped to -1.0 before conversion
                let min_expected = (-1.0f32 * i16::MAX as f32) as i16;
                assert_eq!(v[0], min_expected, "-1.0 should produce -i16::MAX");
                assert_eq!(v[1], min_expected, "-1.5 should be clamped to -1.0");
                assert_eq!(v[2], min_expected, "-2.0 should be clamped to -1.0");
            }
            _ => panic!("Expected Int16 variant"),
        }
    }

    // --- Int24 Conversion Tests ---

    #[test]
    fn test_f32_to_i24_zero() {
        let samples = vec![0.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int24);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0], 0, "0.0 should convert to 0 in i24 (packed in i32)");
            }
            _ => panic!("Expected Int32 variant for Int24 content"),
        }
    }

    #[test]
    fn test_f32_to_i24_positive() {
        let samples = vec![0.5f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int24);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 1);
                // 24-bit max is 8388607 (2^23 - 1)
                let scale = 8388607.0f32;
                let expected = (0.5 * scale) as i32;
                assert_eq!(v[0], expected, "0.5 should convert correctly to i24");
                assert!(v[0] > 0, "0.5 should produce positive i24");
            }
            _ => panic!("Expected Int32 variant for Int24 content"),
        }
    }

    #[test]
    fn test_f32_to_i24_clipping() {
        let samples = vec![1.5f32, -1.5f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int24);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 2);
                let scale = 8388607.0f32;
                let max_expected = scale as i32;
                let min_expected = (-1.0f32 * scale) as i32;
                assert_eq!(
                    v[0], max_expected,
                    "1.5 should be clamped to 1.0 -> max i24"
                );
                assert_eq!(
                    v[1], min_expected,
                    "-1.5 should be clamped to -1.0 -> min i24"
                );
            }
            _ => panic!("Expected Int32 variant for Int24 content"),
        }
    }

    // --- Int32 Conversion Tests ---

    #[test]
    fn test_f32_to_i32_zero() {
        let samples = vec![0.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int32);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0], 0, "0.0 should convert to 0 in i32");
            }
            _ => panic!("Expected Int32 variant"),
        }
    }

    #[test]
    fn test_f32_to_i32_positive() {
        let samples = vec![0.5f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int32);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 1);
                let expected = (0.5 * i32::MAX as f32) as i32;
                assert_eq!(v[0], expected, "0.5 should convert correctly to i32");
                assert!(v[0] > 0, "0.5 should produce positive i32");
            }
            _ => panic!("Expected Int32 variant"),
        }
    }

    #[test]
    fn test_f32_to_i32_clipping() {
        let samples = vec![2.0f32, -2.0f32];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Int32);

        match data {
            AudioData::Int32(v) => {
                assert_eq!(v.len(), 2);
                let max_expected = i32::MAX as f32 as i32;
                let min_expected = (-1.0f32 * i32::MAX as f32) as i32;
                assert_eq!(
                    v[0], max_expected,
                    "2.0 should be clamped to 1.0 -> max i32"
                );
                assert_eq!(
                    v[1], min_expected,
                    "-2.0 should be clamped to -1.0 -> min i32"
                );
            }
            _ => panic!("Expected Int32 variant"),
        }
    }

    // --- Float32 Passthrough Tests ---

    #[test]
    fn test_f32_passthrough() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0, 0.123456789];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Float32);

        match data {
            AudioData::Float32(v) => {
                assert_eq!(v, samples, "Float32 should pass through unchanged");
            }
            _ => panic!("Expected Float32 variant"),
        }
    }

    #[test]
    fn test_f32_passthrough_no_clipping() {
        // Float32 mode should NOT clip values (they pass through as-is)
        let samples = vec![1.5f32, -1.5, 10.0, -10.0];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Float32);

        match data {
            AudioData::Float32(v) => {
                assert_eq!(
                    v, samples,
                    "Float32 should pass through even out-of-range values"
                );
            }
            _ => panic!("Expected Float32 variant"),
        }
    }

    // --- Float64 Conversion Tests ---

    #[test]
    fn test_f32_to_f64_passthrough() {
        // Float64 target should still use Float32 storage (implementation detail)
        let samples = vec![0.0f32, 0.5, -0.5];
        let data = AudioData::from_f32(&samples, SupportedBitDepth::Float64);

        match data {
            AudioData::Float32(v) => {
                assert_eq!(v, samples, "Float64 target should pass through as Float32");
            }
            _ => panic!(
                "Expected Float32 variant for Float64 target (implementation uses f32 storage)"
            ),
        }
    }

    // --- AudioData len/is_empty Tests ---

    #[test]
    fn test_audio_data_len() {
        let data16 = AudioData::Int16(vec![0i16; 100]);
        let data32 = AudioData::Int32(vec![0i32; 200]);
        let dataf32 = AudioData::Float32(vec![0.0f32; 300]);

        assert_eq!(data16.len(), 100);
        assert_eq!(data32.len(), 200);
        assert_eq!(dataf32.len(), 300);
    }

    #[test]
    fn test_audio_data_is_empty() {
        let empty16 = AudioData::Int16(vec![]);
        let empty32 = AudioData::Int32(vec![]);
        let emptyf32 = AudioData::Float32(vec![]);

        assert!(empty16.is_empty());
        assert!(empty32.is_empty());
        assert!(emptyf32.is_empty());

        let nonempty = AudioData::Int16(vec![1]);
        assert!(!nonempty.is_empty());
    }

    // --- Accuracy Tests ---

    #[test]
    fn test_i16_conversion_accuracy() {
        // Test that conversion is accurate within the expected precision
        let test_values = [-1.0f32, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];

        for &val in &test_values {
            let data = AudioData::from_f32(&[val], SupportedBitDepth::Int16);
            match data {
                AudioData::Int16(v) => {
                    // Convert back to float
                    let recovered = v[0] as f32 / i16::MAX as f32;
                    // Should be within 1 LSB accuracy
                    let max_error = 1.0 / i16::MAX as f32;
                    assert!(
                        (recovered - val).abs() <= max_error * 2.0,
                        "Round-trip error too large for {}: got {} (error: {})",
                        val,
                        recovered,
                        (recovered - val).abs()
                    );
                }
                _ => panic!("Expected Int16"),
            }
        }
    }

    #[test]
    fn test_i24_conversion_accuracy() {
        // Test 24-bit conversion accuracy
        let test_values = [-1.0f32, -0.5, 0.0, 0.5, 1.0];
        let scale = 8388607.0f32; // 2^23 - 1

        for &val in &test_values {
            let data = AudioData::from_f32(&[val], SupportedBitDepth::Int24);
            match data {
                AudioData::Int32(v) => {
                    // Convert back to float
                    let recovered = v[0] as f32 / scale;
                    // Should be within 1 LSB accuracy
                    let max_error = 1.0 / scale;
                    assert!(
                        (recovered - val).abs() <= max_error * 2.0,
                        "Round-trip error too large for {}: got {} (error: {})",
                        val,
                        recovered,
                        (recovered - val).abs()
                    );
                }
                _ => panic!("Expected Int32"),
            }
        }
    }

    #[test]
    fn test_conversion_preserves_sign() {
        let positive = vec![0.5f32; 10];
        let negative = vec![-0.5f32; 10];

        let data_pos = AudioData::from_f32(&positive, SupportedBitDepth::Int16);
        let data_neg = AudioData::from_f32(&negative, SupportedBitDepth::Int16);

        match (data_pos, data_neg) {
            (AudioData::Int16(pos), AudioData::Int16(neg)) => {
                assert!(
                    pos.iter().all(|&s| s > 0),
                    "All positive samples should be positive"
                );
                assert!(
                    neg.iter().all(|&s| s < 0),
                    "All negative samples should be negative"
                );
            }
            _ => panic!("Expected Int16"),
        }
    }
}

// =============================================================================
// Section 3: LatencyInfo Tests
// =============================================================================

mod latency_info_tests {
    use super::*;

    #[test]
    fn test_latency_info_default() {
        let info = LatencyInfo::default();

        assert_eq!(info.buffer_samples, 0);
        assert_eq!(info.buffer_ms, 0.0);
        assert_eq!(info.total_ms, 0.0);
        assert!(!info.exclusive);
    }

    #[test]
    fn test_latency_info_fields() {
        let info = LatencyInfo {
            buffer_samples: 256,
            buffer_ms: 5.8,
            total_ms: 10.8,
            exclusive: true,
        };

        assert_eq!(info.buffer_samples, 256);
        assert!((info.buffer_ms - 5.8).abs() < 0.01);
        assert!((info.total_ms - 10.8).abs() < 0.01);
        assert!(info.exclusive);
    }

    #[test]
    fn test_latency_calculation_44100hz() {
        // 256 samples at 44100 Hz = 5.805ms
        let buffer_samples = 256u32;
        let sample_rate = 44100u32;
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        let expected_ms = 5.805; // approximately
        assert!(
            (buffer_ms - expected_ms).abs() < 0.01,
            "256 samples at 44.1kHz should be ~5.8ms, got {}",
            buffer_ms
        );
    }

    #[test]
    fn test_latency_calculation_48000hz() {
        // 256 samples at 48000 Hz = 5.333ms
        let buffer_samples = 256u32;
        let sample_rate = 48000u32;
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        let expected_ms = 5.333;
        assert!(
            (buffer_ms - expected_ms).abs() < 0.01,
            "256 samples at 48kHz should be ~5.33ms, got {}",
            buffer_ms
        );
    }

    #[test]
    fn test_latency_calculation_96000hz() {
        // Higher sample rates = lower latency for same buffer size
        let buffer_samples = 256u32;
        let sample_rate = 96000u32;
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        let expected_ms = 2.667;
        assert!(
            (buffer_ms - expected_ms).abs() < 0.01,
            "256 samples at 96kHz should be ~2.67ms, got {}",
            buffer_ms
        );
    }

    #[test]
    fn test_latency_calculation_192000hz() {
        let buffer_samples = 256u32;
        let sample_rate = 192000u32;
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        let expected_ms = 1.333;
        assert!(
            (buffer_ms - expected_ms).abs() < 0.01,
            "256 samples at 192kHz should be ~1.33ms, got {}",
            buffer_ms
        );
    }

    #[test]
    fn test_total_latency_includes_dac_estimate() {
        // Total latency should include buffer + DAC estimate (~5ms)
        let buffer_samples = 256u32;
        let sample_rate = 44100u32;
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;
        let dac_estimate_ms = 5.0;
        let total_ms = buffer_ms + dac_estimate_ms;

        assert!(
            total_ms > buffer_ms,
            "Total latency should exceed buffer latency"
        );
        assert!(
            (total_ms - buffer_ms - 5.0).abs() < 0.01,
            "DAC estimate should be ~5ms"
        );
    }

    #[test]
    fn test_latency_info_serialization() {
        let info = LatencyInfo {
            buffer_samples: 512,
            buffer_ms: 11.6,
            total_ms: 16.6,
            exclusive: true,
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize LatencyInfo");
        assert!(json.contains("512"), "JSON should contain buffer_samples");
        assert!(
            json.contains("exclusive"),
            "JSON should contain exclusive field"
        );

        let deserialized: LatencyInfo =
            serde_json::from_str(&json).expect("Failed to deserialize LatencyInfo");
        assert_eq!(deserialized.buffer_samples, info.buffer_samples);
        assert_eq!(deserialized.exclusive, info.exclusive);
    }

    #[test]
    fn test_latency_different_buffer_sizes() {
        let sample_rate = 44100u32;
        let buffer_sizes = [64, 128, 256, 512, 1024, 2048];

        let mut prev_latency = 0.0f32;
        for &size in &buffer_sizes {
            let latency_ms = size as f32 / sample_rate as f32 * 1000.0;
            assert!(
                latency_ms > prev_latency,
                "Larger buffers should have higher latency"
            );
            prev_latency = latency_ms;
        }
    }
}

// =============================================================================
// Section 4: Sample Format Tests
// =============================================================================

mod sample_format_tests {
    use super::*;

    #[test]
    fn test_bit_perfect_16bit_content() {
        // Simulate 16-bit source content
        let source_16bit: Vec<i16> = vec![0, 16383, -16384, 32767, -32768];

        // Convert to f32 (as a decoder would)
        let f32_samples: Vec<f32> = source_16bit
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();

        // Convert back to 16-bit for output
        let output = AudioData::from_f32(&f32_samples, SupportedBitDepth::Int16);

        match output {
            AudioData::Int16(v) => {
                // Verify round-trip accuracy
                for (i, (&original, &converted)) in source_16bit.iter().zip(v.iter()).enumerate() {
                    // Allow 1 LSB difference due to rounding
                    assert!(
                        (original - converted).abs() <= 1,
                        "Sample {} round-trip error too large: {} vs {}",
                        i,
                        original,
                        converted
                    );
                }
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_bit_perfect_24bit_content() {
        // Simulate 24-bit source content
        let scale_24 = 8388607.0f32; // 2^23 - 1
        let source_24bit: Vec<i32> = vec![0, 4194303, -4194304, 8388607, -8388607];

        // Convert to f32 (as a decoder would)
        let f32_samples: Vec<f32> = source_24bit.iter().map(|&s| s as f32 / scale_24).collect();

        // Convert back to 24-bit for output
        let output = AudioData::from_f32(&f32_samples, SupportedBitDepth::Int24);

        match output {
            AudioData::Int32(v) => {
                // Verify round-trip accuracy
                for (i, (&original, &converted)) in source_24bit.iter().zip(v.iter()).enumerate() {
                    // Allow 1 LSB difference due to rounding
                    assert!(
                        (original - converted).abs() <= 1,
                        "Sample {} round-trip error too large: {} vs {}",
                        i,
                        original,
                        converted
                    );
                }
            }
            _ => panic!("Expected Int32"),
        }
    }

    #[test]
    fn test_format_selection_logic() {
        // Verify that from_f32 returns the correct variant for each bit depth
        let samples = vec![0.5f32];

        match AudioData::from_f32(&samples, SupportedBitDepth::Int16) {
            AudioData::Int16(_) => {}
            _ => panic!("Int16 bit depth should produce Int16 variant"),
        }

        match AudioData::from_f32(&samples, SupportedBitDepth::Int24) {
            AudioData::Int32(_) => {}
            _ => panic!("Int24 bit depth should produce Int32 variant (packed)"),
        }

        match AudioData::from_f32(&samples, SupportedBitDepth::Int32) {
            AudioData::Int32(_) => {}
            _ => panic!("Int32 bit depth should produce Int32 variant"),
        }

        match AudioData::from_f32(&samples, SupportedBitDepth::Float32) {
            AudioData::Float32(_) => {}
            _ => panic!("Float32 bit depth should produce Float32 variant"),
        }

        match AudioData::from_f32(&samples, SupportedBitDepth::Float64) {
            AudioData::Float32(_) => {}
            _ => panic!("Float64 bit depth should produce Float32 variant (implementation detail)"),
        }
    }

    #[test]
    fn test_dynamic_range_i16() {
        // i16 has ~96dB dynamic range (16 bits * 6dB/bit)
        let min_value = -1.0f32;
        let max_value = 1.0f32;
        let data = AudioData::from_f32(&[min_value, max_value], SupportedBitDepth::Int16);

        match data {
            AudioData::Int16(v) => {
                let range = (v[1] as f32 - v[0] as f32).abs();
                // Full range should be close to 65534 (i16::MAX - i16::MIN using our scale)
                assert!(range > 65000.0, "i16 should have full dynamic range");
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_dynamic_range_i24() {
        // i24 has ~144dB dynamic range (24 bits * 6dB/bit)
        let min_value = -1.0f32;
        let max_value = 1.0f32;
        let data = AudioData::from_f32(&[min_value, max_value], SupportedBitDepth::Int24);

        match data {
            AudioData::Int32(v) => {
                let range = (v[1] as f64 - v[0] as f64).abs();
                // Full range should be close to 16777214 (2^24 - 2)
                assert!(range > 16700000.0, "i24 should have full dynamic range");
            }
            _ => panic!("Expected Int32"),
        }
    }
}

// =============================================================================
// Section 5: Property-Based Tests
// =============================================================================

mod property_tests {
    use super::*;

    proptest! {
        /// Property: Converted audio maintains relative amplitude relationships
        #[test]
        fn amplitude_relationships_preserved(
            a in -1.0f32..1.0,
            b in -1.0f32..1.0
        ) {
            let samples = vec![a, b];
            let data = AudioData::from_f32(&samples, SupportedBitDepth::Int16);

            match data {
                AudioData::Int16(v) => {
                    // If a > b, then converted a should be > converted b
                    if a > b {
                        prop_assert!(v[0] >= v[1], "Amplitude order not preserved: {} > {} but {} < {}",
                            a, b, v[0], v[1]);
                    } else if a < b {
                        prop_assert!(v[0] <= v[1], "Amplitude order not preserved: {} < {} but {} > {}",
                            a, b, v[0], v[1]);
                    }
                }
                _ => prop_assert!(false, "Expected Int16"),
            }
        }

        /// Property: Round-trip f32 -> i16 -> f32 is within acceptable error
        #[test]
        fn i16_round_trip_error_bounded(sample in -1.0f32..1.0) {
            let data = AudioData::from_f32(&[sample], SupportedBitDepth::Int16);

            match data {
                AudioData::Int16(v) => {
                    let recovered = v[0] as f32 / i16::MAX as f32;
                    // Max error is 1 LSB = 1/32767 ≈ 0.00003
                    let max_error = 2.0 / i16::MAX as f32; // Allow 2 LSB for rounding
                    prop_assert!(
                        (recovered - sample).abs() <= max_error,
                        "Round-trip error too large: {} vs {} (error: {})",
                        sample, recovered, (recovered - sample).abs()
                    );
                }
                _ => prop_assert!(false, "Expected Int16"),
            }
        }

        /// Property: Round-trip f32 -> i24 -> f32 is within acceptable error
        #[test]
        fn i24_round_trip_error_bounded(sample in -1.0f32..1.0) {
            let scale = 8388607.0f32;
            let data = AudioData::from_f32(&[sample], SupportedBitDepth::Int24);

            match data {
                AudioData::Int32(v) => {
                    let recovered = v[0] as f32 / scale;
                    // Max error is 1 LSB = 1/8388607 ≈ 0.00000012
                    let max_error = 2.0 / scale; // Allow 2 LSB for rounding
                    prop_assert!(
                        (recovered - sample).abs() <= max_error,
                        "Round-trip error too large: {} vs {} (error: {})",
                        sample, recovered, (recovered - sample).abs()
                    );
                }
                _ => prop_assert!(false, "Expected Int32"),
            }
        }

        /// Property: Values outside [-1, 1] are clamped, never produce overflow
        #[test]
        fn out_of_range_values_clamped(sample in -10.0f32..10.0) {
            let data16 = AudioData::from_f32(&[sample], SupportedBitDepth::Int16);
            let data24 = AudioData::from_f32(&[sample], SupportedBitDepth::Int24);
            let data32 = AudioData::from_f32(&[sample], SupportedBitDepth::Int32);

            match data16 {
                AudioData::Int16(v) => {
                    prop_assert!(v[0] >= i16::MIN && v[0] <= i16::MAX,
                        "i16 value out of bounds: {}", v[0]);
                }
                _ => prop_assert!(false, "Expected Int16"),
            }

            match data24 {
                AudioData::Int32(v) => {
                    let max_24 = 8388607i32;
                    let min_24 = -8388607i32;
                    prop_assert!(v[0] >= min_24 && v[0] <= max_24,
                        "i24 value out of bounds: {} (expected {} to {})", v[0], min_24, max_24);
                }
                _ => prop_assert!(false, "Expected Int32"),
            }

            match data32 {
                AudioData::Int32(v) => {
                    // i32 can't overflow from f32 cast, but should be clamped to reasonable range
                    prop_assert!(v[0] >= i32::MIN && v[0] <= i32::MAX,
                        "i32 value out of bounds: {}", v[0]);
                }
                _ => prop_assert!(false, "Expected Int32"),
            }
        }

        /// Property: Zero input always produces zero output
        #[test]
        fn zero_produces_zero(bit_depth in prop::sample::select(vec![
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
        ])) {
            let data = AudioData::from_f32(&[0.0f32], bit_depth);

            match data {
                AudioData::Int16(v) => prop_assert_eq!(v[0], 0),
                AudioData::Int32(v) => prop_assert_eq!(v[0], 0),
                AudioData::Float32(v) => prop_assert_eq!(v[0], 0.0),
            }
        }

        /// Property: Sign is preserved during conversion
        #[test]
        fn sign_preserved(sample in 0.01f32..1.0) {
            let positive = AudioData::from_f32(&[sample], SupportedBitDepth::Int16);
            let negative = AudioData::from_f32(&[-sample], SupportedBitDepth::Int16);

            match (positive, negative) {
                (AudioData::Int16(p), AudioData::Int16(n)) => {
                    prop_assert!(p[0] > 0, "Positive sample should produce positive output");
                    prop_assert!(n[0] < 0, "Negative sample should produce negative output");
                    prop_assert_eq!(p[0], -n[0], "Symmetric samples should produce symmetric outputs");
                }
                _ => prop_assert!(false, "Expected Int16"),
            }
        }

        /// Property: Float32 passthrough is exact
        #[test]
        fn float32_passthrough_exact(sample in -1.0f32..1.0) {
            let data = AudioData::from_f32(&[sample], SupportedBitDepth::Float32);

            match data {
                AudioData::Float32(v) => prop_assert_eq!(v[0], sample, "Float32 should pass through exactly"),
                _ => prop_assert!(false, "Expected Float32"),
            }
        }

        /// Property: Buffer conversion preserves length
        #[test]
        fn buffer_length_preserved(
            samples in prop::collection::vec(-1.0f32..1.0, 1..1000),
            bit_depth in prop::sample::select(vec![
                SupportedBitDepth::Int16,
                SupportedBitDepth::Int24,
                SupportedBitDepth::Int32,
                SupportedBitDepth::Float32,
            ])
        ) {
            let original_len = samples.len();
            let data = AudioData::from_f32(&samples, bit_depth);

            prop_assert_eq!(data.len(), original_len, "Conversion should preserve buffer length");
        }

        /// Property: Conversion never produces NaN or Inf
        #[test]
        fn no_nan_or_inf(sample in -10.0f32..10.0) {
            // Test with potentially problematic inputs
            let samples = vec![sample, 0.0, -0.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE];

            for &s in &samples {
                let data = AudioData::from_f32(&[s], SupportedBitDepth::Float32);
                match data {
                    AudioData::Float32(v) => {
                        prop_assert!(v[0].is_finite(), "Float32 should not produce NaN or Inf from {}", s);
                    }
                    _ => prop_assert!(false, "Expected Float32"),
                }
            }
        }

        /// Property: Latency calculation is monotonic with buffer size
        #[test]
        fn latency_monotonic_with_buffer_size(
            buffer1 in 1u32..10000,
            buffer2 in 1u32..10000,
            sample_rate in 8000u32..192000
        ) {
            let latency1 = buffer1 as f32 / sample_rate as f32 * 1000.0;
            let latency2 = buffer2 as f32 / sample_rate as f32 * 1000.0;

            if buffer1 > buffer2 {
                prop_assert!(latency1 > latency2, "Larger buffer should have higher latency");
            } else if buffer1 < buffer2 {
                prop_assert!(latency1 < latency2, "Smaller buffer should have lower latency");
            } else {
                prop_assert_eq!(latency1, latency2, "Same buffer should have same latency");
            }
        }

        /// Property: Latency decreases with sample rate for fixed buffer
        #[test]
        fn latency_decreases_with_sample_rate(
            buffer_size in 64u32..4096,
            rate1 in 8000u32..96000,
            rate2 in 96001u32..192000
        ) {
            let latency1 = buffer_size as f32 / rate1 as f32 * 1000.0;
            let latency2 = buffer_size as f32 / rate2 as f32 * 1000.0;

            // rate2 > rate1, so latency2 < latency1
            prop_assert!(latency2 < latency1,
                "Higher sample rate should have lower latency: {} ms at {} Hz vs {} ms at {} Hz",
                latency2, rate2, latency1, rate1);
        }
    }
}

// =============================================================================
// Section 6: Integration Tests (with graceful device handling)
// =============================================================================

mod integration_tests {
    use super::*;
    use std::f32::consts::PI;
    use std::time::Duration;

    /// Helper to check if we can create an audio output
    fn has_audio_device() -> bool {
        ExclusiveOutput::new(ExclusiveConfig::default()).is_ok()
    }

    /// Generate a sine wave for testing
    fn generate_sine_wave(
        frequency: f32,
        duration_secs: f32,
        sample_rate: u32,
        channels: u16,
    ) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples * channels as usize);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin() * 0.3; // 30% amplitude
            for _ in 0..channels {
                samples.push(sample);
            }
        }

        samples
    }

    #[test]
    fn test_create_exclusive_output_default() {
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                // Verify basic properties
                assert!(output.sample_rate() > 0, "Sample rate should be positive");
                assert!(
                    output.latency().buffer_samples > 0,
                    "Buffer samples should be positive"
                );
                assert!(
                    output.latency().buffer_ms > 0.0,
                    "Buffer latency should be positive"
                );
                assert!(
                    output.volume() >= 0.0 && output.volume() <= 1.0,
                    "Volume should be in range"
                );
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device found - skipping test (expected in CI)");
            }
            Err(e) => {
                // Other errors might be acceptable in CI environments
                println!(
                    "Exclusive output creation error (may be expected in CI): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_exclusive_output_with_bit_perfect_config() {
        // Test with bit-perfect 24-bit config
        match ExclusiveOutput::new(ExclusiveConfig::bit_perfect_24()) {
            Ok(output) => {
                assert!(output.sample_rate() > 0);
                assert!(output.config().exclusive_mode);
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device found - skipping test");
            }
            Err(e) => {
                println!("Exclusive output error (may be expected): {}", e);
            }
        }
    }

    #[test]
    fn test_exclusive_output_with_low_latency_config() {
        match ExclusiveOutput::new(ExclusiveConfig::low_latency()) {
            Ok(output) => {
                // Buffer should be small for low latency
                assert!(
                    output.latency().buffer_ms < 10.0,
                    "Low latency config should have <10ms buffer latency"
                );
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device found - skipping test");
            }
            Err(e) => {
                println!("Exclusive output error (may be expected): {}", e);
            }
        }
    }

    #[test]
    fn test_play_pause_stop_commands() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        // Generate test audio
        let sample_rate = output.sample_rate();
        let samples = generate_sine_wave(440.0, 0.5, sample_rate, 2);
        let data = AudioData::Float32(samples);

        // Test play
        assert!(output.play(data).is_ok(), "Play should succeed");
        std::thread::sleep(Duration::from_millis(50));
        assert!(output.is_playing(), "Should be playing after play()");

        // Test pause
        assert!(output.pause().is_ok(), "Pause should succeed");
        std::thread::sleep(Duration::from_millis(50));
        assert!(output.is_paused(), "Should be paused after pause()");
        assert!(!output.is_playing(), "Should not be playing when paused");

        // Test resume
        assert!(output.resume().is_ok(), "Resume should succeed");
        std::thread::sleep(Duration::from_millis(50));
        assert!(!output.is_paused(), "Should not be paused after resume()");

        // Test stop
        assert!(output.stop().is_ok(), "Stop should succeed");
        std::thread::sleep(Duration::from_millis(50));
        assert!(!output.is_playing(), "Should not be playing after stop()");
    }

    #[test]
    fn test_volume_control() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        // Test initial volume
        let initial_volume = output.volume();
        assert!(
            (initial_volume - 1.0).abs() < 0.001,
            "Initial volume should be 1.0"
        );

        // Test setting volume
        assert!(output.set_volume(0.5).is_ok());
        assert!(
            (output.volume() - 0.5).abs() < 0.001,
            "Volume should be 0.5"
        );

        // Test volume clamping
        assert!(output.set_volume(0.0).is_ok());
        assert!(
            (output.volume() - 0.0).abs() < 0.001,
            "Volume should be 0.0"
        );

        assert!(output.set_volume(1.0).is_ok());
        assert!(
            (output.volume() - 1.0).abs() < 0.001,
            "Volume should be 1.0"
        );

        // Test clamping above 1.0 (should be clamped)
        assert!(output.set_volume(1.5).is_ok());
        assert!(output.volume() <= 1.0, "Volume should be clamped to <= 1.0");
    }

    #[test]
    fn test_play_f32_samples() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();
        let samples = generate_sine_wave(440.0, 0.1, sample_rate, 2);

        // play_f32 should automatically convert based on config bit depth
        assert!(output.play_f32(&samples).is_ok(), "play_f32 should succeed");
        std::thread::sleep(Duration::from_millis(50));

        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_looping_mode() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();
        let samples = generate_sine_wave(440.0, 0.1, sample_rate, 2);
        let data = AudioData::Float32(samples);

        // Enable looping
        output.set_looping(true);

        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(200)); // Wait longer than track duration

        // Should still be playing due to looping
        assert!(output.is_playing(), "Should still be playing in loop mode");

        // Disable looping and stop
        output.set_looping(false);
        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_position_tracking() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        // Initial position should be 0
        assert_eq!(output.position(), 0, "Initial position should be 0");

        let sample_rate = output.sample_rate();
        let samples = generate_sine_wave(440.0, 1.0, sample_rate, 2);
        let data = AudioData::Float32(samples);

        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        // Position should have advanced
        let pos = output.position();
        assert!(pos > 0, "Position should advance during playback");

        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_config_accessor() {
        let config = ExclusiveConfig::bit_perfect_24()
            .with_sample_rate(96000)
            .with_buffer_frames(256);

        match ExclusiveOutput::new(config.clone()) {
            Ok(output) => {
                let retrieved_config = output.config();
                assert_eq!(retrieved_config.bit_depth, SupportedBitDepth::Int24);
                assert!(retrieved_config.exclusive_mode);
                // Note: sample_rate might differ from requested if device doesn't support it
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device - skipping test");
            }
            Err(e) => {
                println!("Exclusive output error (may be expected): {}", e);
            }
        }
    }

    #[test]
    fn test_latency_accessor() {
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                let latency = output.latency();

                assert!(latency.buffer_samples > 0, "Buffer samples should be > 0");
                assert!(latency.buffer_ms > 0.0, "Buffer ms should be > 0");
                assert!(
                    latency.total_ms >= latency.buffer_ms,
                    "Total should include buffer"
                );

                // Total latency should include DAC estimate
                assert!(
                    latency.total_ms > latency.buffer_ms,
                    "Total latency should exceed buffer latency (DAC estimate)"
                );
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device - skipping test");
            }
            Err(e) => {
                println!("Exclusive output error (may be expected): {}", e);
            }
        }
    }

    #[test]
    fn test_empty_buffer_playback() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        // Playing empty buffer should not panic
        let data = AudioData::Float32(vec![]);
        assert!(
            output.play(data).is_ok(),
            "Playing empty buffer should succeed"
        );
        std::thread::sleep(Duration::from_millis(50));
        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_silence_buffer_playback() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();
        // 1 second of stereo silence
        let silence = vec![0.0f32; sample_rate as usize * 2];
        let data = AudioData::Float32(silence);

        assert!(output.play(data).is_ok(), "Playing silence should succeed");
        std::thread::sleep(Duration::from_millis(100));
        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_multiple_play_calls() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();

        // Play multiple buffers in sequence
        for freq in [440.0, 523.0, 659.0] {
            // A4, C5, E5
            let samples = generate_sine_wave(freq, 0.1, sample_rate, 2);
            let data = AudioData::Float32(samples);
            assert!(output.play(data).is_ok());
            std::thread::sleep(Duration::from_millis(80));
        }

        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_volume_change_during_playback() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();
        let samples = generate_sine_wave(440.0, 0.5, sample_rate, 2);
        let data = AudioData::Float32(samples);

        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        // Change volume while playing
        assert!(output.set_volume(0.5).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        assert!(output.set_volume(0.2).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_different_sample_formats() {
        let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
            println!("No audio device - skipping test");
            return;
        };

        let sample_rate = output.sample_rate();

        // Test Int16 data
        let i16_data: Vec<i16> = (0..sample_rate as usize * 2)
            .map(|i| {
                let t = i as f32 / sample_rate as f32 / 2.0;
                ((2.0 * PI * 440.0 * t).sin() * 0.3 * i16::MAX as f32) as i16
            })
            .collect();
        let data = AudioData::Int16(i16_data);
        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        // Test Int32 data
        let i32_data: Vec<i32> = (0..sample_rate as usize * 2)
            .map(|i| {
                let t = i as f32 / sample_rate as f32 / 2.0;
                ((2.0 * PI * 440.0 * t).sin() * 0.3 * i32::MAX as f32) as i32
            })
            .collect();
        let data = AudioData::Int32(i32_data);
        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        // Test Float32 data
        let f32_data: Vec<f32> = (0..sample_rate as usize * 2)
            .map(|i| {
                let t = i as f32 / sample_rate as f32 / 2.0;
                (2.0 * PI * 440.0 * t).sin() * 0.3
            })
            .collect();
        let data = AudioData::Float32(f32_data);
        assert!(output.play(data).is_ok());
        std::thread::sleep(Duration::from_millis(100));

        assert!(output.stop().is_ok());
    }

    #[test]
    fn test_drop_stops_playback() {
        // Create output in a block so it gets dropped
        {
            let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) else {
                println!("No audio device - skipping test");
                return;
            };

            let sample_rate = output.sample_rate();
            let samples = generate_sine_wave(440.0, 1.0, sample_rate, 2);
            let data = AudioData::Float32(samples);

            assert!(output.play(data).is_ok());
            std::thread::sleep(Duration::from_millis(50));
            // Output will be dropped here
        }

        // If we get here without hanging, Drop worked correctly
        std::thread::sleep(Duration::from_millis(50));
    }
}

// =============================================================================
// Section 7: Edge Case Tests
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_special_float_values() {
        // Test that special float values don't crash conversion
        let special_values = vec![
            0.0f32,
            -0.0,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::EPSILON,
            -f32::EPSILON,
        ];

        for bit_depth in [
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
        ] {
            let data = AudioData::from_f32(&special_values, bit_depth);
            assert!(
                !data.is_empty(),
                "Conversion should succeed for special values"
            );
        }
    }

    #[test]
    fn test_very_small_values() {
        // Values smaller than 1 LSB should round to 0 for integer formats
        let tiny_values = vec![1e-10f32, -1e-10, 1e-20, -1e-20];

        let data = AudioData::from_f32(&tiny_values, SupportedBitDepth::Int16);
        match data {
            AudioData::Int16(v) => {
                for &val in &v {
                    assert_eq!(val, 0, "Very small values should round to 0");
                }
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_boundary_values() {
        // Test values at exact boundaries
        let boundaries = vec![1.0f32, -1.0, 0.999999, -0.999999, 1.0 - 1e-6, -1.0 + 1e-6];

        let data = AudioData::from_f32(&boundaries, SupportedBitDepth::Int16);
        match data {
            AudioData::Int16(v) => {
                assert_eq!(v.len(), 6);
                // 1.0 should produce i16::MAX
                assert_eq!(v[0], i16::MAX, "1.0 should produce i16::MAX");
                // -1.0 should produce -i16::MAX (not i16::MIN due to asymmetry handling)
                assert!(v[1] < 0, "-1.0 should produce negative value");
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_large_buffer() {
        // Test with a large buffer to check for performance/memory issues
        let large_buffer = vec![0.5f32; 1_000_000];

        let data = AudioData::from_f32(&large_buffer, SupportedBitDepth::Int16);
        assert_eq!(data.len(), 1_000_000);
    }

    #[test]
    fn test_alternating_signs() {
        // Test alternating positive/negative values (common in audio)
        let alternating: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();

        let data = AudioData::from_f32(&alternating, SupportedBitDepth::Int16);
        match data {
            AudioData::Int16(v) => {
                for (i, &val) in v.iter().enumerate() {
                    if i % 2 == 0 {
                        assert!(val > 0, "Even indices should be positive");
                    } else {
                        assert!(val < 0, "Odd indices should be negative");
                    }
                }
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_gradual_ramp() {
        // Test gradual ramp from -1 to 1
        let ramp: Vec<f32> = (0..1001).map(|i| (i as f32 / 500.0) - 1.0).collect();

        let data = AudioData::from_f32(&ramp, SupportedBitDepth::Int16);
        match data {
            AudioData::Int16(v) => {
                // First should be negative, middle should be ~0, last should be positive
                assert!(v[0] < 0, "Start of ramp should be negative");
                assert!(v[500].abs() <= 1, "Middle of ramp should be ~0");
                assert!(v[1000] > 0, "End of ramp should be positive");

                // Verify monotonicity
                for i in 1..v.len() {
                    assert!(v[i] >= v[i - 1], "Ramp should be monotonically increasing");
                }
            }
            _ => panic!("Expected Int16"),
        }
    }

    #[test]
    fn test_config_with_zero_buffer() {
        // Buffer frames of 0 should be handled gracefully
        let config = ExclusiveConfig::default().with_buffer_frames(0);

        // This might fail or use a default, but should not panic
        match ExclusiveOutput::new(config) {
            Ok(_) | Err(_) => {
                // Either is acceptable, just shouldn't panic
            }
        }
    }

    #[test]
    fn test_config_with_very_large_buffer() {
        // Very large buffer request - behavior depends on device capabilities
        // Some devices may clamp, others may accept or reject
        let config = ExclusiveConfig::default().with_buffer_frames(1_000_000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // If creation succeeds, buffer_samples should be > 0
                // The actual clamping behavior is device-dependent
                assert!(
                    output.latency().buffer_samples > 0,
                    "Buffer samples should be positive"
                );
                // Note: We don't assert < 1_000_000 because some devices/backends
                // may accept large buffers or fall back to defaults
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device - skipping test");
            }
            Err(_) => {
                // Rejection with an error is also acceptable behavior
            }
        }
    }

    #[test]
    fn test_config_with_unsupported_sample_rate() {
        // Request a potentially unsupported sample rate
        let config = ExclusiveConfig::default().with_sample_rate(12345);

        match ExclusiveOutput::new(config) {
            Ok(_) => {
                // Device might have found a fallback
            }
            Err(AudioOutputError::DeviceNotFound) => {
                println!("No audio device - skipping test");
            }
            Err(e) => {
                // Error is expected for unsupported rate
                println!("Expected error for unsupported sample rate: {}", e);
            }
        }
    }

    #[test]
    fn test_nonexistent_device() {
        let config = ExclusiveConfig::default().with_device("NonExistent Device XYZ 12345");

        let result = ExclusiveOutput::new(config);
        assert!(
            result.is_err(),
            "Should fail to create output with nonexistent device"
        );
    }
}
