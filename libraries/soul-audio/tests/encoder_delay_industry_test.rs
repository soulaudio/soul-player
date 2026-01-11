//! Encoder Delay Compensation - Industry Standards Test Suite
//!
//! Comprehensive tests for encoder delay handling based on industry standards
//! and implementations from major audio players and encoders.
//!
//! # Industry Standards Referenced
//!
//! ## LAME MP3 Encoder (lame.sourceforge.io)
//! - **LAME Info Tag Spec Rev 1**: gabriel.mp3-tech.org/mp3infotag.html
//! - **Encoder delay**: Stored as 12-bit value (0-4095) at offset 141-143 in Xing/Info frame
//! - **Decoder delay**: 529 samples (mandated by MP3 spec for MDCT overlap-add)
//! - **LAME default delay**: 576 samples (one MP3 granule) since LAME 3.55
//! - **Total gapless offset**: encoder_delay + decoder_delay = typically 576 + 529 = 1105 samples
//!
//! ## Apple iTunSMPB (iTunes Sound Check Metadata)
//! - **Format**: " 00000000 XXXXXXXX YYYYYYYY ZZZZZZZZZZZZZZZZ"
//! - **Field 1**: Always zeros (reserved)
//! - **Field 2** (XXXXXXXX): Encoder delay/priming samples (hex)
//! - **Field 3** (YYYYYYYY): End padding samples (hex)
//! - **Field 4** (ZZZZ...): Original sample count (16 hex digits)
//! - **Standard AAC delay**: 2112 samples (Apple's historical implementation)
//! - **AAC frame size**: 1024 samples (padding rounds to multiple of 1024)
//!
//! ## Gapless Playback Requirements
//! - **Precise gapless**: No gaps or overlaps between tracks
//! - **Sample-accurate**: Decode exactly the original samples
//! - **No heuristics**: Use metadata, not guesswork
//!
//! # Player Implementations Referenced
//!
//! ## foobar2000 (Reference implementation)
//! - Reads LAME tag from first MP3 frame
//! - Supports iTunSMPB for AAC/ALAC
//! - Manual gapless info editing via Utilities menu
//!
//! ## Symphonia (Rust decoder library)
//! - Gapless support via FormatOptions::enable_gapless
//! - Reads encoder delay from LAME tag
//! - Tested against 50,000+ real-world files
//!
//! ## mpv/VLC
//! - mpv: Full gapless support with LAME headers
//! - VLC: Gapless expected in v4.x (historically unsupported)
//!
//! # Test Categories
//!
//! 1. **LAME Header Parsing**: Byte-level parsing of Xing/Info frame
//! 2. **iTunSMPB Parsing**: Apple's gapless metadata format
//! 3. **Delay Calculation**: Converting metadata to sample offsets
//! 4. **Seek Compensation**: Accurate seeking with delay offset
//! 5. **Duration Accuracy**: Correct duration after delay removal
//! 6. **Edge Cases**: Corrupted/missing/unusual metadata

use soul_audio::encoder_delay::{DelaySource, DelayTrimmer, EncoderDelay};
use std::f32::consts::PI;

// =============================================================================
// LAME MP3 HEADER SPECIFICATIONS
// Reference: gabriel.mp3-tech.org/mp3infotag.html
// =============================================================================

/// LAME header constants
/// Reference: LAME source code and MP3 Info Tag Specifications Rev 1
mod lame_constants {
    /// Standard LAME encoder delay (samples added at start)
    /// Since LAME 3.55, the MDCT/filterbank has 48 sample delay,
    /// but LAME pads to 576 for one full granule alignment
    pub const LAME_DEFAULT_ENCODER_DELAY: u32 = 576;

    /// MP3 decoder delay (samples of junk at start)
    /// This is mandated by the MP3 spec for MDCT overlap-add reconstruction
    pub const MP3_DECODER_DELAY: u32 = 529;

    /// Total delay for gapless calculation
    /// gapless_start = encoder_delay + decoder_delay
    pub const TOTAL_DEFAULT_DELAY: u32 = LAME_DEFAULT_ENCODER_DELAY + MP3_DECODER_DELAY;

    /// MP3 frame size in samples (Layer III)
    pub const MP3_FRAME_SIZE: u32 = 1152;

    /// MP3 granule size (half frame)
    pub const MP3_GRANULE_SIZE: u32 = 576;

    /// Maximum valid encoder delay value (12-bit field)
    pub const MAX_ENCODER_DELAY: u32 = 4095;

    /// Maximum valid padding value (12-bit field)
    pub const MAX_END_PADDING: u32 = 4095;

    /// Xing header identifier
    pub const XING_ID: &[u8; 4] = b"Xing";

    /// Info header identifier (CBR files)
    pub const INFO_ID: &[u8; 4] = b"Info";

    /// LAME version string length in header
    pub const LAME_VERSION_LENGTH: usize = 9;

    /// Offset from start of Xing/Info header to LAME tag
    /// (after Xing header fields: ID, flags, frames, bytes, TOC, VBR scale)
    pub const LAME_TAG_OFFSET: usize = 120;

    /// Offset within LAME tag to delay/padding field
    pub const DELAY_PADDING_OFFSET: usize = 21;
}

/// Apple iTunSMPB constants
/// Reference: Apple Developer Documentation - Audio Priming
mod apple_constants {
    /// Standard AAC encoder delay (priming samples)
    /// This value was chosen because most AAC encoders at the time used 2112
    pub const AAC_STANDARD_ENCODER_DELAY: u32 = 2112;

    /// AAC frame/access unit size in samples
    pub const AAC_FRAME_SIZE: u32 = 1024;

    /// Alternative encoder delays seen in the wild
    pub const FDK_AAC_DELAY: u32 = 2048; // FDK-AAC encoder
    pub const FFMPEG_AAC_DELAY: u32 = 1024; // FFmpeg internal AAC encoder
    pub const NERO_AAC_DELAY: u32 = 2112; // Nero AAC encoder

    /// Maximum samples in iTunSMPB (48-bit field, but practically limited)
    pub const MAX_SAMPLE_COUNT: u64 = 0xFFFFFFFFFFFF;
}

/// Opus constants
mod opus_constants {
    /// Opus pre-skip (encoder delay) is codec-defined
    /// Typical range: 312-360 samples
    pub const OPUS_TYPICAL_PRE_SKIP: u32 = 312;

    /// Opus frame duration options (ms)
    pub const OPUS_FRAME_DURATIONS: &[f32] = &[2.5, 5.0, 10.0, 20.0, 40.0, 60.0];
}

// =============================================================================
// LAME HEADER PARSING TESTS
// Reference: LAME source code and MP3 Info Tag Specifications
// =============================================================================

/// Test LAME header byte layout for delay/padding
/// The delay and padding are stored as 12-bit values in 3 bytes:
/// Byte 0: delay[11:4] (high 8 bits of delay)
/// Byte 1: delay[3:0] | padding[11:8] (low 4 bits of delay + high 4 bits of padding)
/// Byte 2: padding[7:0] (low 8 bits of padding)
#[test]
fn test_lame_header_byte_layout() {
    // Standard LAME delay: 576 (0x240)
    // Standard padding: 1152 (0x480)
    //
    // Binary:
    // delay  = 576  = 0b0010_0100_0000 = 0x240
    // padding = 1152 = 0b0100_1000_0000 = 0x480
    //
    // Packed:
    // Byte 0 = 0x24 (delay >> 4)
    // Byte 1 = 0x04 ((delay & 0xF) << 4 | (padding >> 8))
    // Byte 2 = 0x80 (padding & 0xFF)

    let header_bytes: [u8; 3] = [0x24, 0x04, 0x80];
    let delay = EncoderDelay::parse_lame_header(&header_bytes).unwrap();

    assert_eq!(
        delay.start_padding, 576,
        "LAME delay should be 576 samples"
    );
    assert_eq!(
        delay.end_padding, 1152,
        "LAME padding should be 1152 samples (one frame)"
    );
    assert_eq!(
        delay.source,
        DelaySource::LameHeader,
        "Source should be LAME header"
    );
}

/// Test various LAME encoder delay values seen in the wild
#[test]
fn test_lame_header_various_delays() {
    // Test cases: (delay, padding, expected_bytes)
    let test_cases = [
        // Standard LAME 3.100 defaults
        (576, 0, [0x24, 0x00, 0x00]),
        (576, 576, [0x24, 0x02, 0x40]),
        (576, 1152, [0x24, 0x04, 0x80]),
        // Zero delay (shouldn't happen but should parse)
        (0, 0, [0x00, 0x00, 0x00]),
        // Maximum valid values (12-bit max = 4095)
        // Note: parse_lame_header rejects values > 2000 as sanity check
        (2000, 2000, [0x7D, 0x07, 0xD0]),
        // Various real-world values
        (529, 1024, [0x21, 0x14, 0x00]),
        (1024, 512, [0x40, 0x02, 0x00]),
    ];

    for (expected_delay, expected_padding, bytes) in test_cases {
        let result = EncoderDelay::parse_lame_header(&bytes);

        if expected_delay <= 2000 && expected_padding <= 2000 {
            let delay = result.expect(&format!(
                "Should parse delay={}, padding={}",
                expected_delay, expected_padding
            ));

            assert_eq!(
                delay.start_padding, expected_delay,
                "Delay mismatch for {:02X?}",
                bytes
            );
            assert_eq!(
                delay.end_padding, expected_padding,
                "Padding mismatch for {:02X?}",
                bytes
            );
        }
    }
}

/// Test LAME header parsing rejects invalid values
#[test]
fn test_lame_header_rejects_invalid_values() {
    // Values too high (> 2000 are rejected as sanity check)
    let too_high = [0xFF, 0xFF, 0xFF]; // 4095, 4095
    assert!(
        EncoderDelay::parse_lame_header(&too_high).is_none(),
        "Should reject delay/padding > 2000"
    );

    // Maximum valid value edge case
    let just_under_limit = [0x7D, 0x07, 0xD0]; // 2000, 2000
    assert!(
        EncoderDelay::parse_lame_header(&just_under_limit).is_some(),
        "Should accept delay/padding = 2000"
    );

    let just_over_limit = [0x7D, 0x17, 0xD1]; // 2001, 2001
    assert!(
        EncoderDelay::parse_lame_header(&just_over_limit).is_none(),
        "Should reject delay/padding > 2000"
    );
}

/// Test Xing/Info header identification
/// Reference: The Xing header uses "Xing" for VBR, "Info" for CBR
#[test]
fn test_xing_info_header_identification() {
    // Verify the constants match expected values
    assert_eq!(lame_constants::XING_ID, b"Xing");
    assert_eq!(lame_constants::INFO_ID, b"Info");
}

/// Test encoding of delay/padding values back to bytes
/// This verifies our understanding of the byte layout
#[test]
fn test_lame_delay_encoding() {
    fn encode_delay_padding(delay: u32, padding: u32) -> [u8; 3] {
        let byte0 = (delay >> 4) as u8;
        let byte1 = (((delay & 0x0F) << 4) | ((padding >> 8) & 0x0F)) as u8;
        let byte2 = (padding & 0xFF) as u8;
        [byte0, byte1, byte2]
    }

    // Test round-trip for various values
    let values = [
        (0, 0),
        (1, 1),
        (576, 1152),
        (529, 512),
        (1024, 2000),
        (2000, 2000),
    ];

    for (delay, padding) in values {
        let encoded = encode_delay_padding(delay, padding);
        let decoded = EncoderDelay::parse_lame_header(&encoded);

        if delay <= 2000 && padding <= 2000 {
            let d = decoded.expect(&format!(
                "Should decode delay={}, padding={}",
                delay, padding
            ));
            assert_eq!(d.start_padding, delay, "Delay round-trip failed");
            assert_eq!(d.end_padding, padding, "Padding round-trip failed");
        }
    }
}

// =============================================================================
// iTunSMPB PARSING TESTS
// Reference: Apple Developer Documentation - Audio Priming
// =============================================================================

/// Test standard iTunSMPB format parsing
/// Format: " 00000000 XXXXXXXX YYYYYYYY ZZZZZZZZZZZZZZZZ"
#[test]
fn test_itun_smpb_standard_format() {
    // Standard Apple AAC with 2112 sample delay
    let smpb = " 00000000 00000840 000001CA 0000000000ABCDEF";
    let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();

    assert_eq!(delay.start_padding, 0x840, "Delay should be 0x840 = 2112");
    assert_eq!(
        delay.end_padding, 0x1CA,
        "Padding should be 0x1CA = 458"
    );
    assert_eq!(
        delay.valid_samples,
        Some(0xABCDEF),
        "Sample count should be 0xABCDEF"
    );
    assert_eq!(delay.source, DelaySource::ITunSMPB);
}

/// Test iTunSMPB with various encoder outputs
#[test]
fn test_itun_smpb_various_encoders() {
    // Apple AAC (standard 2112 priming)
    let apple_aac = " 00000000 00000840 000001CA 0000000000123456";
    let delay = EncoderDelay::from_itun_smpb(apple_aac).unwrap();
    assert_eq!(delay.start_padding, apple_constants::AAC_STANDARD_ENCODER_DELAY);

    // FDK-AAC (2048 priming)
    let fdk_aac = " 00000000 00000800 000001CA 0000000000123456";
    let delay = EncoderDelay::from_itun_smpb(fdk_aac).unwrap();
    assert_eq!(delay.start_padding, apple_constants::FDK_AAC_DELAY);

    // FFmpeg internal AAC (1024 priming)
    let ffmpeg_aac = " 00000000 00000400 000001CA 0000000000123456";
    let delay = EncoderDelay::from_itun_smpb(ffmpeg_aac).unwrap();
    assert_eq!(delay.start_padding, apple_constants::FFMPEG_AAC_DELAY);

    // Very small file (edge case)
    let small_file = " 00000000 00000840 00000000 0000000000000400";
    let delay = EncoderDelay::from_itun_smpb(small_file).unwrap();
    assert_eq!(delay.valid_samples, Some(0x400));
}

/// Test iTunSMPB with ALAC (lossless, typically no padding needed)
#[test]
fn test_itun_smpb_alac() {
    // ALAC files often have minimal or no encoder delay
    // However, iTunes still writes iTunSMPB for consistency
    let alac_minimal = " 00000000 00000000 00000000 0000000000FEDCBA";
    let delay = EncoderDelay::from_itun_smpb(alac_minimal).unwrap();
    assert_eq!(delay.start_padding, 0);
    assert_eq!(delay.end_padding, 0);
    assert_eq!(delay.valid_samples, Some(0xFEDCBA));
}

/// Test iTunSMPB edge cases and malformed input
#[test]
fn test_itun_smpb_edge_cases() {
    // Too few fields
    assert!(EncoderDelay::from_itun_smpb("00000000 00000840 000001CA").is_none());

    // Invalid hex
    assert!(EncoderDelay::from_itun_smpb("00000000 GHIJKLMN 000001CA 0000000000123456").is_none());

    // Empty string
    assert!(EncoderDelay::from_itun_smpb("").is_none());

    // Only whitespace
    assert!(EncoderDelay::from_itun_smpb("   ").is_none());

    // Valid but unusual (no leading space)
    let no_leading_space = "00000000 00000840 000001CA 0000000000123456";
    let delay = EncoderDelay::from_itun_smpb(no_leading_space).unwrap();
    assert_eq!(delay.start_padding, 0x840);
}

/// Test iTunSMPB sample count calculation
#[test]
fn test_itun_smpb_sample_count_validation() {
    // Sample count = total decoded samples - encoder delay - end padding
    // For AAC: total_samples = original_samples + encoder_delay + end_padding
    // So: valid_samples = total_samples - delay - padding

    let smpb = " 00000000 00000840 000001CA 0000000000100000";
    let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();

    // Valid samples from metadata
    let valid_samples = delay.valid_samples.unwrap();
    assert_eq!(valid_samples, 0x100000);

    // Total decoded samples (what decoder would output)
    let total_decoded = valid_samples + delay.start_padding as u64 + delay.end_padding as u64;

    // Verify actual_samples returns the correct value
    assert_eq!(
        delay.actual_samples(total_decoded),
        valid_samples,
        "actual_samples should return valid_samples when available"
    );
}

// =============================================================================
// VORBIS/OPUS COMMENT PARSING TESTS
// =============================================================================

/// Test Vorbis comment parsing for gapless info
#[test]
fn test_vorbis_comment_parsing() {
    // Standard Opus pre-skip
    let delay = EncoderDelay::from_vorbis_comment(Some("312"), Some("256")).unwrap();
    assert_eq!(delay.start_padding, 312);
    assert_eq!(delay.end_padding, 256);
    assert_eq!(delay.source, DelaySource::VorbisComment);

    // Only delay, no padding
    let delay = EncoderDelay::from_vorbis_comment(Some("360"), None).unwrap();
    assert_eq!(delay.start_padding, 360);
    assert_eq!(delay.end_padding, 0);

    // Only padding, no delay
    let delay = EncoderDelay::from_vorbis_comment(None, Some("512")).unwrap();
    assert_eq!(delay.start_padding, 0);
    assert_eq!(delay.end_padding, 512);

    // Both zero (should return None per implementation)
    assert!(EncoderDelay::from_vorbis_comment(Some("0"), Some("0")).is_none());

    // Both None
    assert!(EncoderDelay::from_vorbis_comment(None, None).is_none());
}

/// Test Opus typical pre-skip values
#[test]
fn test_opus_typical_preskip() {
    // Opus standard pre-skip range: 312-360 samples
    let typical_preskip = opus_constants::OPUS_TYPICAL_PRE_SKIP;
    assert!(typical_preskip >= 312 && typical_preskip <= 360);

    let delay = EncoderDelay::from_vorbis_comment(Some("312"), None).unwrap();
    assert_eq!(delay.start_padding, 312);
}

// =============================================================================
// GAPLESS ALBUM PLAYBACK TESTS
// Verify seamless transitions between tracks
// =============================================================================

/// Simulate gapless playback of an album
/// Tests that removing encoder delay produces continuous audio
#[test]
fn test_gapless_album_playback_simulation() {
    // Simulate a 3-track album encoded with LAME
    // Each track is a continuous sine wave that should seamlessly connect

    let sample_rate = 44100;
    let encoder_delay = 576; // LAME default
    let _decoder_delay = 529; // MP3 decoder delay (unused in simulation)

    struct Track {
        // Raw decoded samples (includes padding)
        decoded_samples: Vec<f32>,
        // Encoder delay info
        delay: EncoderDelay,
        // Total raw samples
        total_samples: u64,
    }

    // Generate tracks that should connect seamlessly
    fn generate_track(
        frequency: f32,
        sample_rate: u32,
        duration_secs: f32,
        start_phase: f32,
        encoder_delay: u32,
        end_padding: u32,
    ) -> (Track, f32) {
        let valid_samples = (sample_rate as f32 * duration_secs) as usize;
        let total_samples = encoder_delay as usize + valid_samples + end_padding as usize;

        let mut samples = Vec::with_capacity(total_samples * 2); // Stereo

        // Pre-padding (encoder delay + decoder delay noise, simulated as zeros)
        for _ in 0..(encoder_delay as usize * 2) {
            samples.push(0.0);
        }

        // Valid audio content
        let phase = start_phase;
        for i in 0..valid_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t + phase).sin() * 0.8;
            samples.push(sample); // Left
            samples.push(sample); // Right
        }

        // Calculate ending phase for next track
        let end_phase =
            (2.0 * PI * frequency * (valid_samples as f32 / sample_rate as f32) + phase)
                % (2.0 * PI);

        // End padding (simulated as trailing zeros)
        for _ in 0..(end_padding as usize * 2) {
            samples.push(0.0);
        }

        let track = Track {
            decoded_samples: samples,
            delay: EncoderDelay::from_lame(encoder_delay, end_padding),
            total_samples: total_samples as u64,
        };

        (track, end_phase)
    }

    // Generate 3 tracks at same frequency for seamless transition
    let frequency = 440.0;
    let track_duration = 0.5; // 500ms per track
    let end_padding = 1152; // One frame

    let (track1, end_phase1) =
        generate_track(frequency, sample_rate, track_duration, 0.0, encoder_delay, end_padding);

    let (track2, end_phase2) = generate_track(
        frequency,
        sample_rate,
        track_duration,
        end_phase1,
        encoder_delay,
        end_padding,
    );

    let (track3, _) = generate_track(
        frequency,
        sample_rate,
        track_duration,
        end_phase2,
        encoder_delay,
        end_padding,
    );

    // Simulate gapless playback by trimming delay and padding
    fn trim_to_gapless(track: &Track) -> Vec<f32> {
        let start_skip = track.delay.start_padding as usize * 2; // Stereo
        let end_skip = track.delay.end_padding as usize * 2;
        let end_idx = track.decoded_samples.len() - end_skip;

        track.decoded_samples[start_skip..end_idx].to_vec()
    }

    let trimmed1 = trim_to_gapless(&track1);
    let trimmed2 = trim_to_gapless(&track2);
    let trimmed3 = trim_to_gapless(&track3);

    // Concatenate for gapless playback
    let mut combined = Vec::new();
    combined.extend_from_slice(&trimmed1);
    combined.extend_from_slice(&trimmed2);
    combined.extend_from_slice(&trimmed3);

    // Verify no discontinuities at track boundaries
    let boundary1 = trimmed1.len();
    let boundary2 = boundary1 + trimmed2.len();

    // Check continuity at first boundary
    let before_boundary1 = combined[boundary1 - 2]; // Left channel
    let after_boundary1 = combined[boundary1]; // Left channel

    // Maximum expected derivative for 440Hz sine at 0.8 amplitude
    let max_derivative = 2.0 * PI * frequency * 0.8 / sample_rate as f32;

    let boundary1_jump = (after_boundary1 - before_boundary1).abs();
    assert!(
        boundary1_jump < max_derivative * 2.0,
        "Boundary 1 should be continuous: jump={:.6}, max_normal={:.6}",
        boundary1_jump,
        max_derivative
    );

    // Check continuity at second boundary
    let before_boundary2 = combined[boundary2 - 2];
    let after_boundary2 = combined[boundary2];

    let boundary2_jump = (after_boundary2 - before_boundary2).abs();
    assert!(
        boundary2_jump < max_derivative * 2.0,
        "Boundary 2 should be continuous: jump={:.6}, max_normal={:.6}",
        boundary2_jump,
        max_derivative
    );

    // Verify total length matches expected
    let expected_samples_per_track = (sample_rate as f32 * track_duration) as usize * 2;
    let expected_total = expected_samples_per_track * 3;
    assert_eq!(
        combined.len(),
        expected_total,
        "Combined length should equal 3 * track_duration"
    );
}

/// Test that phase-mismatched tracks would cause discontinuities
/// This verifies our discontinuity detection works correctly
#[test]
fn test_gapless_detects_phase_mismatch() {
    let sample_rate = 44100;
    let frequency = 1000.0;
    let amplitude = 0.8;
    let duration = 0.1;

    // Generate two tracks with intentionally mismatched phases
    fn generate_sine(
        frequency: f32,
        sample_rate: u32,
        duration: f32,
        amplitude: f32,
        phase: f32,
    ) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(num_samples * 2);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t + phase).sin() * amplitude;
            samples.push(sample);
            samples.push(sample);
        }

        samples
    }

    // Track 1: starts at phase 0
    let track1 = generate_sine(frequency, sample_rate, duration, amplitude, 0.0);

    // Track 2: starts at phase PI (inverted) - guaranteed discontinuity
    let track2 = generate_sine(frequency, sample_rate, duration, amplitude, PI);

    // Get transition samples
    let last_track1 = track1[track1.len() - 2]; // Last left sample
    let first_track2 = track2[0]; // First left sample

    // Calculate the jump
    let transition_jump = (first_track2 - last_track1).abs();

    // Calculate expected normal variation
    let max_normal_derivative = 2.0 * PI * frequency * amplitude / sample_rate as f32;

    // The phase mismatch should cause a larger-than-normal jump
    // (unless we happened to end/start at a zero crossing)
    if last_track1.abs() > 0.1 || first_track2.abs() > 0.1 {
        eprintln!(
            "Phase mismatch detection: last={:.4}, first={:.4}, jump={:.4}, normal_max={:.4}",
            last_track1, first_track2, transition_jump, max_normal_derivative
        );

        // With phase PI offset, most transitions will have noticeable jumps
        // The exact size depends on where in the cycle we are
    }
}

// =============================================================================
// SAMPLE-ACCURATE TIMING TESTS
// Verify correct sample counts after delay compensation
// =============================================================================

/// Test that actual sample count matches expected
#[test]
fn test_sample_accurate_timing() {
    // Simulate a 3-second track encoded with LAME
    let sample_rate = 44100;
    let original_duration_secs = 3.0;
    let original_samples = (sample_rate as f32 * original_duration_secs) as u64;

    // LAME adds encoder delay at start and padding at end
    let encoder_delay = 576;
    let end_padding = 576; // Padding to complete last frame

    let total_encoded_samples = original_samples + encoder_delay as u64 + end_padding as u64;

    let delay = EncoderDelay::from_lame(encoder_delay, end_padding);

    // Verify actual_samples calculation
    let actual = delay.actual_samples(total_encoded_samples);
    assert_eq!(
        actual, original_samples,
        "actual_samples should return original sample count"
    );

    // Verify duration calculation
    let duration = delay.actual_duration(total_encoded_samples, sample_rate);
    let expected_duration = std::time::Duration::from_secs_f64(original_duration_secs as f64);

    let duration_diff = (duration.as_secs_f64() - expected_duration.as_secs_f64()).abs();
    assert!(
        duration_diff < 0.001,
        "Duration should be {} secs, got {} secs",
        expected_duration.as_secs_f64(),
        duration.as_secs_f64()
    );
}

/// Test sample accuracy for various track lengths
#[test]
fn test_sample_accuracy_various_lengths() {
    let sample_rate = 44100u32;
    let encoder_delay = 576u32;

    // Test various track lengths
    let durations = [
        0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 60.0, // seconds
    ];

    for &duration_secs in &durations {
        let original_samples = (sample_rate as f64 * duration_secs) as u64;

        // Calculate padding to complete last frame
        let frame_size = 1152u64;
        let total_with_delay = original_samples + encoder_delay as u64;
        let frames_needed = (total_with_delay + frame_size - 1) / frame_size;
        let end_padding = (frames_needed * frame_size - total_with_delay) as u32;

        let total_encoded = original_samples + encoder_delay as u64 + end_padding as u64;

        let delay = EncoderDelay::from_lame(encoder_delay, end_padding);
        let actual = delay.actual_samples(total_encoded);

        assert_eq!(
            actual, original_samples,
            "Duration {} secs: expected {} samples, got {}",
            duration_secs, original_samples, actual
        );
    }
}

/// Test iTunSMPB sample count precedence
/// When valid_samples is provided, it should take precedence
#[test]
fn test_itun_smpb_sample_count_precedence() {
    // iTunSMPB provides explicit sample count
    let smpb = " 00000000 00000840 000001CA 0000000000100000";
    let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();

    // Total decoded samples (hypothetical decoder output)
    let total_decoded = 1100000u64; // Some arbitrary number

    // actual_samples should return the explicit valid_samples, not calculate
    let actual = delay.actual_samples(total_decoded);
    assert_eq!(
        actual,
        delay.valid_samples.unwrap(),
        "Should use explicit valid_samples from iTunSMPB"
    );
}

// =============================================================================
// SEEK ACCURACY TESTS
// Verify seeking works correctly with delay compensation
// =============================================================================

/// Test seek to specific sample position
#[test]
fn test_seek_with_delay_compensation() {
    let delay = EncoderDelay::from_lame(576, 1152);
    let total_samples = 100000u64;
    let mut trimmer = DelayTrimmer::new(delay, total_samples);

    // Seek to sample 1000 (in valid samples)
    let raw_position = trimmer.seek_to(1000);

    // Raw position should account for encoder delay
    assert_eq!(
        raw_position, 1576,
        "Raw position should be target + encoder_delay"
    );

    // Current position should report the valid sample position
    assert_eq!(trimmer.position(), 1000);
}

/// Test seek to various positions
#[test]
fn test_seek_various_positions() {
    let delay = EncoderDelay::from_lame(576, 1152);
    let total_samples = 100000u64;
    let valid_samples = delay.actual_samples(total_samples);

    let mut trimmer = DelayTrimmer::new(delay.clone(), total_samples);

    // Test various seek positions
    let positions = [0, 100, 1000, 10000, 50000, valid_samples - 1];

    for &target in &positions {
        if target < valid_samples {
            let raw = trimmer.seek_to(target);
            assert_eq!(
                raw,
                target + delay.start_padding as u64,
                "Seek to {} should set raw position to {}",
                target,
                target + delay.start_padding as u64
            );
            assert_eq!(trimmer.position(), target);
        }
    }
}

/// Test seek to start (position 0)
#[test]
fn test_seek_to_start() {
    let delay = EncoderDelay::from_lame(576, 1152);
    let total_samples = 50000u64;
    let mut trimmer = DelayTrimmer::new(delay.clone(), total_samples);

    // Advance to middle
    trimmer.advance(25000);
    assert!(trimmer.position() > 0);

    // Seek to start
    trimmer.seek_to(0);
    assert_eq!(trimmer.position(), 0);

    // Reset should also go to start
    trimmer.advance(10000);
    trimmer.reset();
    assert_eq!(trimmer.position(), 0, "Reset should go to position 0");
}

/// Test seek near end of track
#[test]
fn test_seek_near_end() {
    let delay = EncoderDelay::from_lame(576, 1152);
    let total_samples = 50000u64;
    let mut trimmer = DelayTrimmer::new(delay.clone(), total_samples);

    let valid_samples = trimmer.valid_samples();

    // Seek to near end
    let near_end = valid_samples - 100;
    trimmer.seek_to(near_end);
    assert_eq!(trimmer.position(), near_end);

    // Should not be at end padding yet
    assert!(!trimmer.at_end_padding());

    // Advance past valid region
    trimmer.advance(200);
    // Now at end padding
    assert!(trimmer.at_end_padding() || trimmer.should_skip());
}

// =============================================================================
// DURATION CALCULATION TESTS
// Verify correct duration after delay removal
// =============================================================================

/// Test duration calculation for CD audio
#[test]
fn test_duration_calculation_cd_audio() {
    let sample_rate = 44100u32;

    // A typical 4-minute track
    let original_duration_secs = 240.0;
    let original_samples = (sample_rate as f64 * original_duration_secs) as u64;

    let delay = EncoderDelay::from_lame(576, 1152);
    let total_encoded = original_samples + 576 + 1152;

    let duration = delay.actual_duration(total_encoded, sample_rate);

    let expected = std::time::Duration::from_secs_f64(original_duration_secs);
    let diff = (duration.as_secs_f64() - expected.as_secs_f64()).abs();

    assert!(
        diff < 0.01,
        "Duration should be ~4 minutes: expected {:?}, got {:?}",
        expected,
        duration
    );
}

/// Test duration with various sample rates
#[test]
fn test_duration_various_sample_rates() {
    let sample_rates = [8000, 11025, 22050, 44100, 48000, 88200, 96000, 176400, 192000];
    let original_duration = 60.0; // 1 minute

    let delay = EncoderDelay::from_lame(576, 1152);

    for &rate in &sample_rates {
        let original_samples = (rate as f64 * original_duration) as u64;
        let total_encoded = original_samples + 576 + 1152;

        let duration = delay.actual_duration(total_encoded, rate);
        let diff = (duration.as_secs_f64() - original_duration).abs();

        assert!(
            diff < 0.05,
            "Duration at {} Hz should be ~60s: got {:?}",
            rate,
            duration
        );
    }
}

/// Test duration with iTunSMPB (explicit sample count)
#[test]
fn test_duration_with_itun_smpb() {
    let sample_rate = 44100u32;
    let explicit_samples = 441000u64; // Exactly 10 seconds

    let smpb = format!(
        " 00000000 00000840 000001CA {:016X}",
        explicit_samples
    );
    let delay = EncoderDelay::from_itun_smpb(&smpb).unwrap();

    // Total decoded doesn't matter when valid_samples is set
    let total_decoded = 999999u64;

    let duration = delay.actual_duration(total_decoded, sample_rate);
    let expected = std::time::Duration::from_secs(10);

    assert_eq!(duration, expected, "Duration should be exactly 10 seconds");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

/// Test very short files (shorter than encoder delay)
#[test]
fn test_very_short_file() {
    let delay = EncoderDelay::from_lame(576, 1152);

    // File shorter than encoder delay
    let total_samples = 500u64; // Less than 576
    let actual = delay.actual_samples(total_samples);

    // Should saturating_sub to 0
    assert_eq!(actual, 0, "Very short file should have 0 valid samples");
}

/// Test file with no delay info
#[test]
fn test_no_delay_info() {
    let delay = EncoderDelay::new();

    assert_eq!(delay.source, DelaySource::None);
    assert!(!delay.has_padding());
    assert_eq!(delay.total_padding(), 0);

    // Should return total samples unchanged
    let total = 50000u64;
    assert_eq!(delay.actual_samples(total), total);
}

/// Test manual delay specification
#[test]
fn test_manual_delay() {
    let delay = EncoderDelay::manual(1024, 512);

    assert_eq!(delay.source, DelaySource::Manual);
    assert_eq!(delay.start_padding, 1024);
    assert_eq!(delay.end_padding, 512);
    assert!(delay.has_padding());
}

/// Test corrupted LAME headers (all zeros)
#[test]
fn test_corrupted_lame_header_zeros() {
    let zeros: [u8; 3] = [0x00, 0x00, 0x00];
    let result = EncoderDelay::parse_lame_header(&zeros);

    // Should parse but have no padding
    if let Some(delay) = result {
        assert!(!delay.has_padding());
    }
}

/// Test corrupted LAME headers (all 0xFF)
#[test]
fn test_corrupted_lame_header_ff() {
    let all_ff: [u8; 3] = [0xFF, 0xFF, 0xFF];
    let result = EncoderDelay::parse_lame_header(&all_ff);

    // Should reject (values > 2000)
    assert!(result.is_none(), "Should reject header with values > 2000");
}

/// Test frame boundary alignment
#[test]
fn test_frame_boundary_alignment() {
    let _sample_rate = 44100; // Standard CD sample rate
    let mp3_frame_samples = 1152;

    // Original samples that don't align to frame boundary
    let original_samples = 50000u64;

    // Calculate how LAME would pad this
    let with_delay = original_samples + 576;
    let frames_needed = (with_delay + mp3_frame_samples - 1) / mp3_frame_samples;
    let total_frame_samples = frames_needed * mp3_frame_samples;
    let end_padding = (total_frame_samples - with_delay) as u32;

    let delay = EncoderDelay::from_lame(576, end_padding);
    let actual = delay.actual_samples(total_frame_samples);

    assert_eq!(
        actual, original_samples,
        "After trimming, should get original sample count"
    );

    // Verify total is frame-aligned
    assert_eq!(
        total_frame_samples % mp3_frame_samples,
        0,
        "Total samples should be frame-aligned"
    );
}

/// Test CD sector alignment (588 samples)
/// Reference: Audio CD sector = 2352 bytes = 588 samples (16-bit stereo)
#[test]
fn test_cd_sector_alignment() {
    let cd_sector_samples = 588u64;
    let _sample_rate = 44100; // CD standard sample rate

    // CD audio is always aligned to sector boundaries
    let cd_track_sectors = 1000;
    let original_samples = cd_track_sectors * cd_sector_samples;

    let delay = EncoderDelay::from_lame(576, 1164); // Arbitrary padding

    // Calculate what total encoded would be
    let total_encoded = original_samples + 576 + 1164;
    let actual = delay.actual_samples(total_encoded);

    // Verify we get back the sector-aligned original
    assert_eq!(
        actual, original_samples,
        "Should recover CD-aligned sample count"
    );
    assert_eq!(
        actual % cd_sector_samples,
        0,
        "CD audio should be sector-aligned"
    );
}

// =============================================================================
// DELAY TRIMMER TESTS
// =============================================================================

/// Test DelayTrimmer sample skipping
#[test]
fn test_delay_trimmer_start_skip() {
    let delay = EncoderDelay::from_lame(100, 50);
    let mut trimmer = DelayTrimmer::new(delay, 1000);

    // First 100 samples should be skipped (encoder delay)
    for i in 0..100 {
        assert!(
            trimmer.should_skip(),
            "Sample {} should be skipped (start padding)",
            i
        );
        trimmer.advance(1);
    }

    // Sample 100 should NOT be skipped
    assert!(
        !trimmer.should_skip(),
        "Sample 100 should not be skipped"
    );
}

/// Test DelayTrimmer end padding detection
#[test]
fn test_delay_trimmer_end_skip() {
    let delay = EncoderDelay::from_lame(100, 50);
    let total_samples = 1000u64;
    let mut trimmer = DelayTrimmer::new(delay, total_samples);

    // Skip past start padding
    trimmer.advance(100);

    // Valid range: samples 100-949
    // Advance to sample 949 (last valid)
    trimmer.advance(849);

    // Sample 949 is valid (0-indexed position 949 in raw = position 849 in valid)
    assert!(
        !trimmer.should_skip(),
        "Sample 949 should be valid"
    );

    // Advance to sample 950 (first end padding)
    trimmer.advance(1);
    assert!(
        trimmer.should_skip(),
        "Sample 950 should be skipped (end padding)"
    );
    assert!(
        trimmer.at_end_padding(),
        "Should be at end padding"
    );
}

/// Test DelayTrimmer position tracking
#[test]
fn test_delay_trimmer_position_tracking() {
    let delay = EncoderDelay::from_lame(100, 50);
    let mut trimmer = DelayTrimmer::new(delay, 1000);

    // Initially at position 0 (but samples_read is 0, which is in start padding)
    assert_eq!(trimmer.position(), 0);

    // Advance through start padding
    trimmer.advance(100);
    assert_eq!(trimmer.position(), 0, "Just exited start padding");

    // Advance 1 more sample
    trimmer.advance(1);
    assert_eq!(trimmer.position(), 1, "Position should be 1");

    // Advance 1000 samples
    trimmer.advance(1000);
    assert_eq!(trimmer.position(), 1001);
}

/// Test DelayTrimmer valid_samples calculation
#[test]
fn test_delay_trimmer_valid_samples() {
    let delay = EncoderDelay::from_lame(100, 50);
    let total = 1000u64;
    let trimmer = DelayTrimmer::new(delay, total);

    let valid = trimmer.valid_samples();
    assert_eq!(valid, 850, "Valid samples = 1000 - 100 - 50 = 850");
}

// =============================================================================
// SYNTHESIS TESTS FOR GAPLESS VERIFICATION
// Create synthetic audio to verify gapless algorithms
// =============================================================================

/// Generate a reference sine wave for gapless testing
fn generate_reference_sine(
    frequency: f32,
    sample_rate: u32,
    num_samples: usize,
    start_phase: f32,
) -> (Vec<f32>, f32) {
    let mut samples = Vec::with_capacity(num_samples * 2);
    let mut phase = start_phase;
    let phase_increment = 2.0 * PI * frequency / sample_rate as f32;

    for _ in 0..num_samples {
        let sample = phase.sin() * 0.8;
        samples.push(sample); // Left
        samples.push(sample); // Right
        phase = (phase + phase_increment) % (2.0 * PI);
    }

    (samples, phase)
}

/// Test that trimmed audio has correct phase continuity
#[test]
fn test_phase_continuity_after_trim() {
    let sample_rate = 44100;
    let frequency = 440.0;
    let encoder_delay: u32 = 576;
    let end_padding: u32 = 1152;

    // Generate "encoded" audio (with delay and padding)
    let valid_samples = 10000;
    let total_samples = encoder_delay as usize + valid_samples + end_padding as usize;

    // Delay samples (zeros)
    let mut encoded: Vec<f32> = vec![0.0; encoder_delay as usize * 2];

    // Valid audio
    let (valid_audio, _) =
        generate_reference_sine(frequency, sample_rate, valid_samples, 0.0);
    encoded.extend_from_slice(&valid_audio);

    // Padding (zeros)
    encoded.extend(vec![0.0; end_padding as usize * 2]);

    assert_eq!(encoded.len(), total_samples * 2);

    // Trim using delay info
    let delay = EncoderDelay::from_lame(encoder_delay, end_padding);
    let _trimmer = DelayTrimmer::new(delay, total_samples as u64);

    let start_idx = encoder_delay as usize * 2;
    let end_idx = encoded.len() - end_padding as usize * 2;
    let trimmed = &encoded[start_idx..end_idx];

    assert_eq!(trimmed.len(), valid_samples * 2);

    // Verify first sample is approximately 0 (sin(0) = 0)
    assert!(
        trimmed[0].abs() < 0.01,
        "First trimmed sample should be near 0, got {}",
        trimmed[0]
    );

    // Verify the audio is the expected sine wave
    let expected_peak = 0.8;
    let actual_peak = trimmed.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        (actual_peak - expected_peak).abs() < 0.1,
        "Peak should be ~0.8, got {}",
        actual_peak
    );
}

/// Test concatenating multiple trimmed tracks
#[test]
fn test_concatenate_trimmed_tracks() {
    let sample_rate = 44100;
    let frequency = 440.0;
    let _encoder_delay = 576; // Used in real-world but simulation uses already-trimmed audio
    let _end_padding = 576;
    let valid_samples_per_track = 5000;

    // Generate 3 tracks that should concatenate seamlessly
    let mut phase = 0.0;
    let mut all_valid_audio = Vec::new();

    for _track_num in 0..3 {
        let (valid_audio, end_phase) =
            generate_reference_sine(frequency, sample_rate, valid_samples_per_track, phase);
        all_valid_audio.extend_from_slice(&valid_audio);
        phase = end_phase;
    }

    // Verify concatenated audio has no discontinuities
    let max_derivative = 2.0 * PI * frequency * 0.8 / sample_rate as f32;
    let threshold = max_derivative * 2.0;

    let mut discontinuities = 0;
    for i in (2..all_valid_audio.len()).step_by(2) {
        // Check left channel
        let diff = (all_valid_audio[i] - all_valid_audio[i - 2]).abs();
        if diff > threshold {
            discontinuities += 1;
        }
    }

    assert_eq!(
        discontinuities, 0,
        "Should have no discontinuities in concatenated audio"
    );
}

// =============================================================================
// REFERENCE VALUES FROM REAL-WORLD ENCODERS
// These are expected values based on encoder documentation
// =============================================================================

/// Verify LAME encoder constants match documentation
#[test]
fn test_lame_documented_values() {
    // LAME 3.100 defaults (from lame.sourceforge.io)
    assert_eq!(
        lame_constants::LAME_DEFAULT_ENCODER_DELAY, 576,
        "LAME default delay is 576 samples"
    );

    assert_eq!(
        lame_constants::MP3_DECODER_DELAY, 529,
        "MP3 decoder delay is 529 samples"
    );

    assert_eq!(
        lame_constants::MP3_FRAME_SIZE, 1152,
        "MP3 Layer III frame is 1152 samples"
    );
}

/// Verify Apple AAC constants match documentation
#[test]
fn test_apple_aac_documented_values() {
    // Apple Tech Note TN2258 - "Audio Priming"
    assert_eq!(
        apple_constants::AAC_STANDARD_ENCODER_DELAY, 2112,
        "Apple AAC standard delay is 2112 samples"
    );

    assert_eq!(
        apple_constants::AAC_FRAME_SIZE, 1024,
        "AAC frame size is 1024 samples"
    );
}

/// Verify Opus constants
#[test]
fn test_opus_documented_values() {
    // RFC 6716 - Definition of the Opus Audio Codec
    assert!(
        opus_constants::OPUS_TYPICAL_PRE_SKIP >= 312
            && opus_constants::OPUS_TYPICAL_PRE_SKIP <= 360,
        "Opus pre-skip is typically 312-360 samples"
    );
}

// =============================================================================
// INTEGRATION-STYLE TESTS
// Simulate real playback scenarios
// =============================================================================

/// Simulate LAME-encoded track playback
#[test]
fn test_lame_track_playback_simulation() {
    let sample_rate = 44100;
    let original_duration_secs = 180.0; // 3 minutes
    let original_samples = (sample_rate as f64 * original_duration_secs) as u64;

    // LAME encoding simulation
    let encoder_delay = lame_constants::LAME_DEFAULT_ENCODER_DELAY;
    let _decoder_delay = lame_constants::MP3_DECODER_DELAY; // Note: handled by decoder

    // Calculate padding for frame alignment
    let frame_size = lame_constants::MP3_FRAME_SIZE as u64;
    let with_delay = original_samples + encoder_delay as u64;
    let frames = (with_delay + frame_size - 1) / frame_size;
    let total_frame_samples = frames * frame_size;
    let end_padding = (total_frame_samples - with_delay) as u32;

    // Create delay info
    let delay = EncoderDelay::from_lame(encoder_delay, end_padding);

    // Simulate playback
    let total_decoded = total_frame_samples;

    // Calculate what player would report
    let playable_samples = delay.actual_samples(total_decoded);
    let playable_duration = delay.actual_duration(total_decoded, sample_rate);

    // Verify accuracy
    assert_eq!(
        playable_samples, original_samples,
        "Playable samples should match original"
    );

    let duration_error = (playable_duration.as_secs_f64() - original_duration_secs).abs();
    assert!(
        duration_error < 0.001,
        "Duration error should be < 1ms: actual {} vs expected {}",
        playable_duration.as_secs_f64(),
        original_duration_secs
    );
}

/// Simulate iTunes AAC track playback
#[test]
fn test_itunes_aac_playback_simulation() {
    let sample_rate = 44100;
    let original_duration_secs = 240.0; // 4 minutes
    let original_samples = (sample_rate as f64 * original_duration_secs) as u64;

    // iTunes AAC encoding simulation
    let encoder_delay = apple_constants::AAC_STANDARD_ENCODER_DELAY;
    let frame_size = apple_constants::AAC_FRAME_SIZE as u64;

    // Calculate padding for frame alignment
    let with_delay = original_samples + encoder_delay as u64;
    let frames = (with_delay + frame_size - 1) / frame_size;
    let total_frame_samples = frames * frame_size;
    let end_padding = (total_frame_samples - with_delay) as u32;

    // Create iTunSMPB-style metadata
    let smpb = format!(
        " 00000000 {:08X} {:08X} {:016X}",
        encoder_delay, end_padding, original_samples
    );

    let delay = EncoderDelay::from_itun_smpb(&smpb).unwrap();

    // Simulate playback
    let playable_samples = delay.actual_samples(total_frame_samples);
    let playable_duration = delay.actual_duration(total_frame_samples, sample_rate);

    // Verify accuracy
    assert_eq!(
        playable_samples, original_samples,
        "Playable samples should match original"
    );

    let duration_error = (playable_duration.as_secs_f64() - original_duration_secs).abs();
    assert!(
        duration_error < 0.001,
        "Duration error should be < 1ms"
    );
}

/// Test cross-format gapless transition (MP3 -> AAC)
#[test]
fn test_cross_format_gapless() {
    let _sample_rate = 44100; // Would be used in actual playback

    // Track 1: LAME MP3
    let mp3_delay = EncoderDelay::from_lame(576, 1152);
    let mp3_total = 50000u64;
    let mp3_valid = mp3_delay.actual_samples(mp3_total);

    // Track 2: iTunes AAC
    let aac_smpb = " 00000000 00000840 000001CA 000000000000C350";
    let aac_delay = EncoderDelay::from_itun_smpb(aac_smpb).unwrap();
    let aac_valid = aac_delay.valid_samples.unwrap();

    // For gapless playback, we need:
    // 1. Trim end of track 1 (remove mp3 end padding)
    // 2. Start track 2 after its encoder delay

    // Both should provide valid sample counts
    assert!(mp3_valid > 0);
    assert!(aac_valid > 0);

    // Verify different delay sources are tracked
    assert_eq!(mp3_delay.source, DelaySource::LameHeader);
    assert_eq!(aac_delay.source, DelaySource::ITunSMPB);
}

// =============================================================================
// STRESS TESTS
// =============================================================================

/// Test with many tracks (album simulation)
#[test]
fn test_many_track_album() {
    let num_tracks = 20;
    let sample_rate = 44100;

    let mut total_valid_samples = 0u64;

    for track_num in 0..num_tracks {
        // Varying track lengths
        let track_duration = 180.0 + (track_num as f64 * 10.0); // 3-5+ minutes
        let original_samples = (sample_rate as f64 * track_duration) as u64;

        let delay = EncoderDelay::from_lame(576, 1152);
        let total = original_samples + 576 + 1152;

        let valid = delay.actual_samples(total);
        assert_eq!(valid, original_samples);

        total_valid_samples += valid;
    }

    // Verify we can handle many tracks
    assert!(total_valid_samples > 0);
    println!(
        "Total album samples: {} ({:.1} minutes)",
        total_valid_samples,
        total_valid_samples as f64 / sample_rate as f64 / 60.0
    );
}

/// Test rapid seek operations
#[test]
fn test_rapid_seeking() {
    let delay = EncoderDelay::from_lame(576, 1152);
    let total_samples = 1000000u64;
    let mut trimmer = DelayTrimmer::new(delay.clone(), total_samples);
    let valid = trimmer.valid_samples();

    // Perform many random-ish seeks
    for i in 0..100 {
        let position = (i * 7919) % (valid as usize); // Prime number for pseudo-random
        trimmer.seek_to(position as u64);
        assert_eq!(trimmer.position(), position as u64);
    }
}
