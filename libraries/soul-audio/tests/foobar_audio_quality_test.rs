//! Comprehensive Audio Quality Tests
//!
//! Tests for foobar2000-style audio processing features:
//! - TPDF dithering quality
//! - Encoder delay compensation
//! - Signal chain order verification
//!
//! Reference: AES-17, ITU-R BS.1770-4, LAME specification

use soul_audio::dither::TpdfDither;
use soul_audio::encoder_delay::{DelaySource, DelayTrimmer, EncoderDelay};

// ============================================================================
// TPDF DITHERING TESTS
// ============================================================================

mod dither_quality {
    use super::*;

    /// Test that TPDF dither has triangular distribution
    /// Reference: Lipshitz & Vanderkooy - "Dithering" (AES)
    #[test]
    fn test_tpdf_triangular_distribution() {
        let mut dither = TpdfDither::with_seed(42);
        let mut histogram = vec![0i32; 65536];
        let n_samples = 1_000_000;

        // Generate dithered samples from 0.0 and histogram the results
        for _ in 0..n_samples {
            let sample = dither.dither_to_i16(0.0);
            let idx = (sample as i32 + 32768) as usize;
            histogram[idx] += 1;
        }

        // TPDF should peak around zero and taper linearly
        // Check that the distribution is peaked at center
        let center_mass: i32 = histogram[32766..=32770].iter().sum();
        let total_mass: i32 = histogram.iter().sum();

        // Center 5 bins should contain significant portion (> 80% for tight TPDF)
        let center_ratio = center_mass as f64 / total_mass as f64;
        assert!(
            center_ratio > 0.80,
            "TPDF not centered enough: {:.2}% in center bins",
            center_ratio * 100.0
        );
    }

    /// Test that dither noise has zero mean
    /// Reference: Vanderkooy & Lipshitz - "Resolution Below the LSB"
    #[test]
    fn test_dither_zero_mean() {
        let mut dither = TpdfDither::new();
        let n_samples = 100_000;
        let mut sum: i64 = 0;

        for _ in 0..n_samples {
            sum += dither.dither_to_i16(0.5) as i64;
        }

        let mean = sum as f64 / n_samples as f64;
        let expected = 0.5 * 32767.0;

        // Mean should be close to expected (within 1 LSB)
        assert!(
            (mean - expected).abs() < 2.0,
            "Mean {} not close to expected {}",
            mean,
            expected
        );
    }

    /// Test that independent ditherers are decorrelated
    /// Reference: Prevention of phantom center artifacts
    #[test]
    fn test_stereo_decorrelation() {
        // Use two ditherers with different seeds (simulating L/R channels)
        let mut dither_l = TpdfDither::with_seed(12345);
        let mut dither_r = TpdfDither::with_seed(67890);
        let n_samples = 10_000;

        let mut l_samples = Vec::with_capacity(n_samples);
        let mut r_samples = Vec::with_capacity(n_samples);

        for _ in 0..n_samples {
            l_samples.push(dither_l.dither_to_i16(0.5) as f64);
            r_samples.push(dither_r.dither_to_i16(0.5) as f64);
        }

        // Calculate correlation coefficient
        let mean_l: f64 = l_samples.iter().sum::<f64>() / n_samples as f64;
        let mean_r: f64 = r_samples.iter().sum::<f64>() / n_samples as f64;

        let mut cov: f64 = 0.0;
        let mut var_l: f64 = 0.0;
        let mut var_r: f64 = 0.0;

        for i in 0..n_samples {
            let dl = l_samples[i] - mean_l;
            let dr = r_samples[i] - mean_r;
            cov += dl * dr;
            var_l += dl * dl;
            var_r += dr * dr;
        }

        let correlation: f64 = cov / (var_l.sqrt() * var_r.sqrt());

        // Correlation should be low (< 0.1)
        assert!(
            correlation.abs() < 0.1,
            "Stereo channels too correlated: {:.4}",
            correlation
        );
    }

    /// Test that dither doesn't cause DC offset
    #[test]
    fn test_no_dc_offset() {
        let mut dither = TpdfDither::new();
        let n_samples = 100_000;

        // Process alternating +/- samples
        let mut sum: i64 = 0;
        for i in 0..n_samples {
            let input = if i % 2 == 0 { 0.3 } else { -0.3 };
            sum += dither.dither_to_i16(input) as i64;
        }

        // Sum should be near zero (DC component)
        let dc_component = (sum as f64 / n_samples as f64).abs();
        assert!(
            dc_component < 5.0,
            "DC offset too high: {:.2} LSB",
            dc_component
        );
    }

    /// Test 24-bit dithering precision (I32 with 24-bit audio)
    #[test]
    fn test_i32_dither_precision() {
        let mut dither = TpdfDither::new();

        // Small signal that should be preserved with 24-bit precision
        let small_signal = 1.0 / 8388608.0; // 1 LSB at 24-bit

        let mut preserved = 0;
        for _ in 0..1000 {
            let result = dither.dither_to_i32(small_signal);
            if result != 0 {
                preserved += 1;
            }
        }

        // With proper dithering, some samples should be non-zero
        assert!(
            preserved > 100,
            "24-bit dither not preserving small signals: {} non-zero",
            preserved
        );
    }
}

// ============================================================================
// ENCODER DELAY COMPENSATION TESTS
// ============================================================================

mod encoder_delay_tests {
    use super::*;

    /// Test LAME header parsing with known values
    /// Reference: LAME source code - lame.h
    #[test]
    fn test_lame_header_parsing_standard() {
        // Standard LAME encoder delay: 576 samples
        // Byte layout: delay_hi:8, delay_lo:4|pad_hi:4, pad_lo:8
        // 576 = 0x240, 1152 = 0x480
        // Bytes: [0x24, 0x04, 0x80]
        let header = [0x24, 0x04, 0x80];
        let delay = EncoderDelay::parse_lame_header(&header).unwrap();

        assert_eq!(delay.start_padding, 576);
        assert_eq!(delay.end_padding, 1152);
        assert_eq!(delay.source, DelaySource::LameHeader);
    }

    /// Test LAME header with zero padding
    #[test]
    fn test_lame_header_zero_padding() {
        let header = [0x00, 0x00, 0x00];
        let delay = EncoderDelay::parse_lame_header(&header).unwrap();

        assert_eq!(delay.start_padding, 0);
        assert_eq!(delay.end_padding, 0);
    }

    /// Test iTunSMPB parsing with real-world format
    /// Reference: Apple Technical Note - gapless playback
    #[test]
    fn test_itun_smpb_parsing() {
        // Typical iTunSMPB from iTunes-encoded AAC
        // Format: " 00000000 00000840 00000920 0000000000052A20"
        let smpb = " 00000000 00000840 00000920 0000000000052A20";
        let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();

        assert_eq!(delay.start_padding, 0x840); // 2112 samples (AAC encoder delay)
        assert_eq!(delay.end_padding, 0x920);   // 2336 samples
        assert_eq!(delay.valid_samples, Some(0x52A20)); // 338464 samples
        assert_eq!(delay.source, DelaySource::ITunSMPB);
    }

    /// Test delay trimmer correctly skips start padding
    #[test]
    fn test_trimmer_skips_start() {
        let delay = EncoderDelay::from_lame(576, 0);
        let mut trimmer = DelayTrimmer::new(delay, 10000);

        // First 576 samples should be skipped
        for i in 0..576 {
            assert!(trimmer.should_skip(), "Sample {} should be skipped", i);
            trimmer.advance(1);
        }

        // Sample 576 and onwards should NOT be skipped
        assert!(!trimmer.should_skip(), "Sample 576 should not be skipped");
    }

    /// Test delay trimmer correctly skips end padding
    #[test]
    fn test_trimmer_skips_end() {
        let delay = EncoderDelay::from_lame(0, 1152);
        let mut trimmer = DelayTrimmer::new(delay, 10000);

        // Total=10000, end_padding=1152
        // Valid range: [0, 8848) - samples 0 to 8847 are valid
        // valid_end = 10000 - 1152 = 8848
        // Sample 8847 is the last valid sample
        // Sample 8848 is the first end padding sample

        // Advance to last valid sample
        trimmer.advance(8847);
        assert!(!trimmer.should_skip(), "Sample 8847 should not be skipped (last valid)");

        // Advance into end padding
        trimmer.advance(1);
        // Now samples_read = 8848, which >= valid_end (8848)
        assert!(trimmer.should_skip(), "Sample 8848 should be skipped (first end padding)");
    }

    /// Test valid sample count calculation
    #[test]
    fn test_valid_sample_count() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let total_samples = 10000_u64;

        let valid = delay.actual_samples(total_samples);
        assert_eq!(valid, 10000 - 576 - 1152);
    }

    /// Test seek position calculation
    #[test]
    fn test_trimmer_seek() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let mut trimmer = DelayTrimmer::new(delay, 10000);

        // Seek to valid sample 1000
        let raw_pos = trimmer.seek_to(1000);

        // Raw position should include start padding offset
        assert_eq!(raw_pos, 1000 + 576);
        assert_eq!(trimmer.position(), 1000);
    }

    /// Test duration calculation with encoder delay
    #[test]
    fn test_duration_with_delay() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let total_samples = 44100_u64; // 1 second at 44.1kHz
        let sample_rate = 44100_u32;

        let duration = delay.actual_duration(total_samples, sample_rate);

        // Valid samples: 44100 - 576 - 1152 = 42372
        // Duration: 42372 / 44100 ≈ 0.961 seconds
        let expected = (44100.0 - 576.0 - 1152.0) / 44100.0;
        assert!(
            (duration.as_secs_f64() - expected).abs() < 0.001,
            "Duration {} != expected {}",
            duration.as_secs_f64(),
            expected
        );
    }
}

// ============================================================================
// SIGNAL CHAIN ORDER TESTS
// ============================================================================
// Note: Signal chain tests requiring soul_loudness are in soul-loudness crate

// ============================================================================
// HEADROOM MANAGEMENT TESTS
// ============================================================================
// Note: Headroom tests are in soul-loudness crate (headroom module tests)

// ============================================================================
// SAMPLE RATE MODE TESTS
// ============================================================================
// Note: Sample rate mode tests are in soul-audio-desktop crate

// ============================================================================
// RESAMPLER BACKEND TESTS
// ============================================================================

mod resampler_backend_tests {
    use soul_audio::resampling::ResamplerBackend;

    /// Test backend availability check
    #[test]
    fn test_backend_availability() {
        // Rubato should always be available
        let backends = ResamplerBackend::available_backends();
        assert!(backends.contains(&ResamplerBackend::Auto));
        assert!(backends.contains(&ResamplerBackend::Rubato));

        // r8brain availability depends on feature flag
        // Just verify the check doesn't panic
        let _ = ResamplerBackend::r8brain_available();
    }

    /// Test backend resolution
    #[test]
    fn test_backend_resolution() {
        // Auto should resolve to a concrete backend
        let resolved = ResamplerBackend::Auto.resolved();
        assert_ne!(resolved, ResamplerBackend::Auto);

        // Specific backends resolve to themselves
        assert_eq!(ResamplerBackend::Rubato.resolved(), ResamplerBackend::Rubato);
    }

    /// Test backend string conversion
    #[test]
    fn test_backend_string_roundtrip() {
        for backend in [ResamplerBackend::Auto, ResamplerBackend::Rubato, ResamplerBackend::R8Brain] {
            let s = backend.as_str();
            let parsed = ResamplerBackend::from_str(s);
            assert_eq!(parsed, Some(backend));
        }
    }
}
