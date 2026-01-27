//! Encoder delay compensation for gapless playback
//!
//! Parses and applies encoder delay/padding information from:
//! - LAME MP3 headers (encoder delay + padding in Xing/Info frame)
//! - iTunSMPB atoms (AAC/ALAC files with iTunes-style padding info)
//! - Vorbis comments (Opus/Vorbis files)
//!
//! # Background
//!
//! Most audio encoders add padding samples to the beginning and/or end
//! of encoded audio:
//! - **Encoder delay**: Samples added at start for codec warm-up
//! - **End padding**: Samples added to complete the final frame
//!
//! Without compensating for this padding, you hear:
//! - Clicks/silence at track boundaries
//! - Gaps in gapless albums
//! - Incorrect track lengths
//!
//! # Example
//!
//! ```
//! use soul_audio::encoder_delay::{EncoderDelay, DelaySource};
//!
//! // Parse from LAME header
//! let delay = EncoderDelay::from_lame(576, 1152);
//! assert_eq!(delay.start_padding, 576);
//! assert_eq!(delay.end_padding, 1152);
//!
//! // Parse from iTunSMPB
//! let itunes = EncoderDelay::from_itun_smpb(" 00000000 00000840 00000AAC 0000000000012345");
//! assert_eq!(itunes.unwrap().start_padding, 2112);  // 0x840 = 2112
//! ```

/// Source of encoder delay information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DelaySource {
    /// No delay information available
    #[default]
    None,
    /// LAME encoder (from Xing/Info header)
    LameHeader,
    /// iTunes-style SMPB atom (AAC/ALAC)
    ITunSMPB,
    /// Vorbis comment (Opus/Vorbis)
    VorbisComment,
    /// Manually specified
    Manual,
}

/// Encoder delay and padding information
///
/// Contains the number of samples to skip at the beginning and end
/// of a decoded audio stream to achieve sample-accurate playback.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncoderDelay {
    /// Samples to skip at start (encoder delay)
    pub start_padding: u32,
    /// Samples to skip at end (end padding)
    pub end_padding: u32,
    /// Total valid samples (if known)
    pub valid_samples: Option<u64>,
    /// Source of this delay information
    pub source: DelaySource,
}

impl EncoderDelay {
    /// Create a new encoder delay with no padding
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from LAME-style delay values
    ///
    /// # Arguments
    /// * `encoder_delay` - Samples of encoder delay at start
    /// * `end_padding` - Samples of padding at end
    pub fn from_lame(encoder_delay: u32, end_padding: u32) -> Self {
        Self {
            start_padding: encoder_delay,
            end_padding,
            valid_samples: None,
            source: DelaySource::LameHeader,
        }
    }

    /// Parse from LAME header bytes
    ///
    /// The LAME header stores delay info at bytes 141-143 of the Xing/Info frame:
    /// - Bits 0-11: Encoder delay (samples to skip at start)
    /// - Bits 12-23: End padding (samples to skip at end)
    ///
    /// # Arguments
    /// * `header_bytes` - The 3-byte delay/padding field from LAME header
    ///
    /// # Returns
    /// Parsed encoder delay, or None if bytes are invalid
    pub fn parse_lame_header(header_bytes: &[u8; 3]) -> Option<Self> {
        // LAME stores 12-bit encoder delay and 12-bit padding
        // Byte layout: [delay_hi:8][delay_lo:4|padding_hi:4][padding_lo:8]
        let encoder_delay = ((header_bytes[0] as u32) << 4) | ((header_bytes[1] as u32) >> 4);
        let end_padding = (((header_bytes[1] as u32) & 0x0F) << 8) | (header_bytes[2] as u32);

        // Sanity check: values should be reasonable for MP3
        // Typical LAME delay is 576 samples (one MP3 granule)
        // Maximum padding is 1152 (one MP3 frame)
        if encoder_delay > 2000 || end_padding > 2000 {
            return None;
        }

        Some(Self {
            start_padding: encoder_delay,
            end_padding,
            valid_samples: None,
            source: DelaySource::LameHeader,
        })
    }

    /// Parse from iTunSMPB metadata string
    ///
    /// Format: " 00000000 XXXXXXXX YYYYYYYY ZZZZZZZZZZZZZZZZ"
    /// - First field: Always zeros
    /// - Second field (XXXXXXXX): Encoder delay in hex (start padding)
    /// - Third field (YYYYYYYY): End padding in hex
    /// - Fourth field (ZZZZ...): Valid sample count in hex
    ///
    /// # Arguments
    /// * `smpb` - The iTunSMPB string value
    ///
    /// # Returns
    /// Parsed encoder delay, or None if string is invalid
    pub fn from_itun_smpb(smpb: &str) -> Option<Self> {
        // Split on whitespace
        let parts: Vec<&str> = smpb.split_whitespace().collect();

        if parts.len() < 4 {
            return None;
        }

        // Parse hex values
        let start_padding = u32::from_str_radix(parts[1], 16).ok()?;
        let end_padding = u32::from_str_radix(parts[2], 16).ok()?;
        let valid_samples = u64::from_str_radix(parts[3], 16).ok();

        Some(Self {
            start_padding,
            end_padding,
            valid_samples,
            source: DelaySource::ITunSMPB,
        })
    }

    /// Parse from Vorbis/Opus comment
    ///
    /// Opus files use ENCODER_DELAY and ENCODER_PADDING comments.
    /// Format: Simple decimal numbers.
    ///
    /// # Arguments
    /// * `delay` - ENCODER_DELAY value (or similar)
    /// * `padding` - ENCODER_PADDING value (or similar)
    pub fn from_vorbis_comment(delay: Option<&str>, padding: Option<&str>) -> Option<Self> {
        let start_padding = delay.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let end_padding = padding.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

        if start_padding == 0 && end_padding == 0 {
            return None;
        }

        Some(Self {
            start_padding,
            end_padding,
            valid_samples: None,
            source: DelaySource::VorbisComment,
        })
    }

    /// Create manual encoder delay
    pub fn manual(start_padding: u32, end_padding: u32) -> Self {
        Self {
            start_padding,
            end_padding,
            valid_samples: None,
            source: DelaySource::Manual,
        }
    }

    /// Check if there is any padding to compensate for
    pub fn has_padding(&self) -> bool {
        self.start_padding > 0 || self.end_padding > 0
    }

    /// Get the total padding (start + end)
    pub fn total_padding(&self) -> u32 {
        self.start_padding + self.end_padding
    }

    /// Calculate actual duration in samples
    ///
    /// # Arguments
    /// * `total_decoded_samples` - Total samples from decoder
    ///
    /// # Returns
    /// Actual playable samples after removing padding
    pub fn actual_samples(&self, total_decoded_samples: u64) -> u64 {
        if let Some(valid) = self.valid_samples {
            return valid;
        }

        total_decoded_samples.saturating_sub(self.total_padding() as u64)
    }

    /// Calculate actual duration
    ///
    /// # Arguments
    /// * `total_decoded_samples` - Total samples from decoder
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Returns
    /// Actual playable duration
    pub fn actual_duration(
        &self,
        total_decoded_samples: u64,
        sample_rate: u32,
    ) -> std::time::Duration {
        let samples = self.actual_samples(total_decoded_samples);
        std::time::Duration::from_secs_f64(samples as f64 / sample_rate as f64)
    }
}

/// Encoder delay trimmer for applying delay compensation during playback
///
/// Tracks the current position and automatically skips padding samples.
#[derive(Debug, Clone)]
pub struct DelayTrimmer {
    delay: EncoderDelay,
    samples_read: u64,
    total_samples: u64,
}

impl DelayTrimmer {
    /// Create a new delay trimmer
    ///
    /// # Arguments
    /// * `delay` - Encoder delay information
    /// * `total_samples` - Total samples in the stream
    pub fn new(delay: EncoderDelay, total_samples: u64) -> Self {
        Self {
            delay,
            samples_read: 0,
            total_samples,
        }
    }

    /// Check if we should skip the current sample
    ///
    /// Call this for each sample read from the decoder.
    /// Returns true if the sample should be skipped (is padding).
    pub fn should_skip(&self) -> bool {
        // Skip start padding
        if self.samples_read < self.delay.start_padding as u64 {
            return true;
        }

        // Skip end padding
        let valid_end = self
            .total_samples
            .saturating_sub(self.delay.end_padding as u64);
        if self.samples_read >= valid_end {
            return true;
        }

        false
    }

    /// Advance the sample counter
    pub fn advance(&mut self, samples: u64) {
        self.samples_read += samples;
    }

    /// Get the number of samples remaining to skip at start
    pub fn start_samples_to_skip(&self) -> u64 {
        self.delay.start_padding as u64 - self.samples_read.min(self.delay.start_padding as u64)
    }

    /// Check if we're past the start padding
    pub fn past_start_padding(&self) -> bool {
        self.samples_read >= self.delay.start_padding as u64
    }

    /// Check if we've reached the end padding
    pub fn at_end_padding(&self) -> bool {
        let valid_end = self
            .total_samples
            .saturating_sub(self.delay.end_padding as u64);
        self.samples_read >= valid_end
    }

    /// Reset the trimmer (e.g., after seeking to start)
    pub fn reset(&mut self) {
        self.samples_read = 0;
    }

    /// Seek to a specific sample position
    ///
    /// # Arguments
    /// * `sample` - The sample position (in valid samples, not raw samples)
    ///
    /// # Returns
    /// The raw sample position to seek to in the decoder
    pub fn seek_to(&mut self, sample: u64) -> u64 {
        // Convert valid sample position to raw position
        let raw_position = sample + self.delay.start_padding as u64;
        self.samples_read = raw_position;
        raw_position
    }

    /// Get current position in valid samples
    pub fn position(&self) -> u64 {
        self.samples_read
            .saturating_sub(self.delay.start_padding as u64)
    }

    /// Get total valid samples
    pub fn valid_samples(&self) -> u64 {
        self.delay.actual_samples(self.total_samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_delay_default() {
        let delay = EncoderDelay::new();
        assert_eq!(delay.start_padding, 0);
        assert_eq!(delay.end_padding, 0);
        assert!(!delay.has_padding());
    }

    #[test]
    fn test_from_lame() {
        let delay = EncoderDelay::from_lame(576, 1152);
        assert_eq!(delay.start_padding, 576);
        assert_eq!(delay.end_padding, 1152);
        assert_eq!(delay.source, DelaySource::LameHeader);
        assert!(delay.has_padding());
        assert_eq!(delay.total_padding(), 1728);
    }

    #[test]
    fn test_parse_lame_header() {
        // LAME header with delay=576 (0x240), padding=1152 (0x480)
        // Byte layout: [0x24][0x04][0x80]
        let header = [0x24, 0x04, 0x80];
        let delay = EncoderDelay::parse_lame_header(&header).unwrap();
        assert_eq!(delay.start_padding, 576);
        assert_eq!(delay.end_padding, 1152);
    }

    #[test]
    fn test_parse_lame_header_invalid() {
        // Values too high
        let header = [0xFF, 0xFF, 0xFF];
        assert!(EncoderDelay::parse_lame_header(&header).is_none());
    }

    #[test]
    fn test_from_itun_smpb() {
        // Typical iTunSMPB format
        let smpb = " 00000000 00000840 00000AAC 0000000000012345";
        let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();
        assert_eq!(delay.start_padding, 0x840); // 2112
        assert_eq!(delay.end_padding, 0xAAC); // 2732
        assert_eq!(delay.valid_samples, Some(0x12345));
        assert_eq!(delay.source, DelaySource::ITunSMPB);
    }

    #[test]
    fn test_from_itun_smpb_invalid() {
        assert!(EncoderDelay::from_itun_smpb("invalid").is_none());
        assert!(EncoderDelay::from_itun_smpb("00 01 02").is_none());
    }

    #[test]
    fn test_from_vorbis_comment() {
        let delay = EncoderDelay::from_vorbis_comment(Some("312"), Some("256")).unwrap();
        assert_eq!(delay.start_padding, 312);
        assert_eq!(delay.end_padding, 256);
        assert_eq!(delay.source, DelaySource::VorbisComment);
    }

    #[test]
    fn test_from_vorbis_comment_none() {
        assert!(EncoderDelay::from_vorbis_comment(None, None).is_none());
        assert!(EncoderDelay::from_vorbis_comment(Some("0"), Some("0")).is_none());
    }

    #[test]
    fn test_actual_samples() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let total = 10000;
        assert_eq!(delay.actual_samples(total), 10000 - 576 - 1152);
    }

    #[test]
    fn test_actual_samples_with_valid_samples() {
        let delay = EncoderDelay {
            start_padding: 576,
            end_padding: 1152,
            valid_samples: Some(8000),
            source: DelaySource::ITunSMPB,
        };
        assert_eq!(delay.actual_samples(10000), 8000);
    }

    #[test]
    fn test_delay_trimmer_start_skip() {
        let delay = EncoderDelay::from_lame(100, 50);
        let mut trimmer = DelayTrimmer::new(delay, 1000);

        // First 100 samples should be skipped
        for i in 0..100 {
            assert!(trimmer.should_skip(), "Sample {} should be skipped", i);
            trimmer.advance(1);
        }

        // Next sample should not be skipped
        assert!(!trimmer.should_skip());
    }

    #[test]
    fn test_delay_trimmer_end_skip() {
        let delay = EncoderDelay::from_lame(100, 50);
        let mut trimmer = DelayTrimmer::new(delay, 1000);

        // Advance past start padding to last valid sample before end padding
        // Total=1000, start=100, end=50
        // Valid range: [100, 950) -> samples 100-949 are valid
        trimmer.advance(949);

        // Should not skip yet (at sample 949, which is valid)
        assert!(!trimmer.should_skip());

        // Advance into end padding (sample 950, where end padding starts)
        trimmer.advance(1);
        assert!(trimmer.should_skip());
    }

    #[test]
    fn test_delay_trimmer_seek() {
        let delay = EncoderDelay::from_lame(100, 50);
        let mut trimmer = DelayTrimmer::new(delay, 1000);

        // Seek to valid sample 500
        let raw_pos = trimmer.seek_to(500);
        assert_eq!(raw_pos, 600); // 500 + 100 start padding
        assert_eq!(trimmer.position(), 500);
    }

    #[test]
    fn test_delay_trimmer_valid_samples() {
        let delay = EncoderDelay::from_lame(100, 50);
        let trimmer = DelayTrimmer::new(delay, 1000);
        assert_eq!(trimmer.valid_samples(), 850); // 1000 - 100 - 50
    }

    #[test]
    fn test_actual_duration() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let duration = delay.actual_duration(44100, 44100);
        // 44100 - 576 - 1152 = 42372 samples = ~0.961 seconds
        assert!((duration.as_secs_f64() - 0.961).abs() < 0.01);
    }

    // =============================================================================
    // Edge Case Tests for Industry Standards Compliance
    // =============================================================================

    /// Test Opus seek convergence requirement (RFC 7845)
    /// After seeking, Opus recommends skipping 80ms (3840 samples at 48kHz) for convergence
    #[test]
    fn test_opus_seek_convergence_requirement() {
        // Opus at 48kHz with typical pre-skip
        let pre_skip = 312u32;
        let seek_convergence_samples = 3840u32; // 80ms at 48kHz

        let delay = EncoderDelay::from_vorbis_comment(Some("312"), None).unwrap();
        assert_eq!(delay.start_padding, pre_skip);

        // After seeking mid-file, player should skip max(pre_skip, seek_convergence)
        // This is typically handled by the decoder, but delay info should support it
        let effective_skip_after_seek = seek_convergence_samples.max(pre_skip);
        assert_eq!(effective_skip_after_seek, 3840);
    }

    /// Test MP3 combined encoder + decoder delay
    /// Total gapless offset = encoder_delay (576) + decoder_delay (529) = 1105 samples
    #[test]
    fn test_mp3_combined_encoder_decoder_delay() {
        const LAME_ENCODER_DELAY: u32 = 576;
        const MP3_DECODER_DELAY: u32 = 529;
        const TOTAL_DELAY: u32 = LAME_ENCODER_DELAY + MP3_DECODER_DELAY;

        // LAME stores only encoder delay, decoder adds its own
        let delay = EncoderDelay::from_lame(LAME_ENCODER_DELAY, 1152);
        assert_eq!(delay.start_padding, LAME_ENCODER_DELAY);

        // When calculating total skip at start, player should add decoder delay
        let total_start_skip = delay.start_padding + MP3_DECODER_DELAY;
        assert_eq!(total_start_skip, TOTAL_DELAY);
        assert_eq!(total_start_skip, 1105);
    }

    /// Test HE-AAC (SBR) extended delay
    /// HE-AAC has additional delay due to Spectral Band Replication
    #[test]
    fn test_he_aac_extended_delay() {
        // Standard AAC LC: 2112 samples
        // HE-AAC (SBR): 2048 + additional SBR delay
        // HE-AACv2 (SBR+PS): Even more delay

        // Common HE-AAC delay values observed in the wild
        let he_aac_delays = [2048u32, 2304, 2560, 3072];

        for &delay_value in &he_aac_delays {
            let smpb = format!(" 00000000 {:08X} 00000200 0000000000100000", delay_value);
            let delay = EncoderDelay::from_itun_smpb(&smpb).unwrap();
            assert_eq!(delay.start_padding, delay_value);
            // HE-AAC delays should be within reasonable range
            assert!((1024..=4096).contains(&delay_value));
        }
    }

    /// Test LAME header at maximum valid values (12-bit max = 4095)
    /// Implementation rejects > 2000 as sanity check
    #[test]
    fn test_lame_header_boundary_values() {
        // At the sanity check boundary (2000)
        let at_limit = [0x7D, 0x07, 0xD0]; // 2000, 2000
        let delay = EncoderDelay::parse_lame_header(&at_limit).unwrap();
        assert_eq!(delay.start_padding, 2000);
        assert_eq!(delay.end_padding, 2000);

        // Just over the limit (2001)
        let over_limit = [0x7D, 0x17, 0xD1]; // 2001, 2001
        assert!(EncoderDelay::parse_lame_header(&over_limit).is_none());

        // At zero
        let zeros = [0x00, 0x00, 0x00];
        let delay = EncoderDelay::parse_lame_header(&zeros).unwrap();
        assert_eq!(delay.start_padding, 0);
        assert_eq!(delay.end_padding, 0);
        assert!(!delay.has_padding());
    }

    /// Test iTunSMPB with very large sample counts
    #[test]
    fn test_itun_smpb_large_sample_count() {
        // 16-bit hex field can represent up to 2^64 - 1 samples
        // Test with a 1-hour file at 192kHz (691,200,000 samples)
        let sample_count = 691_200_000u64;
        let smpb = format!(" 00000000 00000840 000001CA {:016X}", sample_count);
        let delay = EncoderDelay::from_itun_smpb(&smpb).unwrap();
        assert_eq!(delay.valid_samples, Some(sample_count));
    }

    /// Test iTunSMPB with maximum encoder delays
    #[test]
    fn test_itun_smpb_maximum_delays() {
        // Maximum 32-bit values (though unrealistic)
        let smpb = " 00000000 FFFFFFFF FFFFFFFF 0000000000100000";
        let delay = EncoderDelay::from_itun_smpb(smpb).unwrap();
        assert_eq!(delay.start_padding, 0xFFFFFFFF);
        assert_eq!(delay.end_padding, 0xFFFFFFFF);
    }

    /// Test cross-format transition: MP3 to AAC
    /// Both formats have different delay characteristics
    #[test]
    fn test_cross_format_mp3_to_aac() {
        // Track 1: MP3 with LAME
        let mp3_delay = EncoderDelay::from_lame(576, 1152);
        let mp3_total = 50000u64;
        let mp3_valid = mp3_delay.actual_samples(mp3_total);

        // Track 2: AAC with iTunSMPB
        let aac_delay =
            EncoderDelay::from_itun_smpb(" 00000000 00000840 000001CA 000000000000C350").unwrap();
        let _aac_valid = aac_delay.valid_samples.unwrap();

        // Verify different sources are correctly tracked
        assert_eq!(mp3_delay.source, DelaySource::LameHeader);
        assert_eq!(aac_delay.source, DelaySource::ITunSMPB);

        // MP3 valid samples calculation
        assert_eq!(mp3_valid, 50000 - 576 - 1152);
    }

    /// Test file shorter than total padding (edge case)
    #[test]
    fn test_file_shorter_than_padding() {
        let delay = EncoderDelay::from_lame(1000, 1000);

        // Total samples less than start + end padding
        let total = 1500u64;
        let actual = delay.actual_samples(total);

        // Should saturating_sub to 0
        assert_eq!(actual, 0);
    }

    /// Test DelayTrimmer with position exactly at boundaries
    #[test]
    fn test_delay_trimmer_exact_boundaries() {
        let delay = EncoderDelay::from_lame(100, 100);
        let total = 500u64;
        let mut trimmer = DelayTrimmer::new(delay, total);

        // Position exactly at start of valid region
        trimmer.advance(100);
        assert!(trimmer.past_start_padding());
        assert!(!trimmer.should_skip());
        assert_eq!(trimmer.position(), 0);

        // Position exactly at end of valid region
        // Valid samples = 500 - 100 - 100 = 300
        // Valid range: samples 100-399 (raw), positions 0-299 (valid)
        trimmer.advance(299);
        assert!(!trimmer.should_skip());
        assert_eq!(trimmer.position(), 299);

        // One more puts us in end padding
        trimmer.advance(1);
        assert!(trimmer.at_end_padding());
        assert!(trimmer.should_skip());
    }

    /// Test DelayTrimmer seek to exact boundaries
    #[test]
    fn test_delay_trimmer_seek_boundaries() {
        let delay = EncoderDelay::from_lame(100, 100);
        let total = 500u64;
        let mut trimmer = DelayTrimmer::new(delay, total);

        let valid = trimmer.valid_samples(); // 300

        // Seek to start
        let raw = trimmer.seek_to(0);
        assert_eq!(raw, 100); // Account for start padding
        assert_eq!(trimmer.position(), 0);

        // Seek to last valid sample
        let raw = trimmer.seek_to(valid - 1);
        assert_eq!(raw, 100 + valid - 1);
        assert_eq!(trimmer.position(), valid - 1);
    }

    /// Test Opus pre-skip range validation
    #[test]
    fn test_opus_preskip_range() {
        // Typical Opus pre-skip is 312-360 samples at 48kHz
        for preskip in [312, 324, 336, 348, 360] {
            let delay =
                EncoderDelay::from_vorbis_comment(Some(&preskip.to_string()), None).unwrap();
            assert_eq!(delay.start_padding, preskip);
            assert_eq!(delay.source, DelaySource::VorbisComment);
        }
    }

    /// Test AAC frame alignment (1024 samples)
    #[test]
    fn test_aac_frame_alignment() {
        const AAC_FRAME_SIZE: u64 = 1024;

        // Original samples that don't align to frame boundary
        let original_samples = 50000u64;
        let encoder_delay = 2112u32; // Standard Apple AAC

        // Calculate padding to complete last frame
        let with_delay = original_samples + encoder_delay as u64;
        let frames_needed = with_delay.div_ceil(AAC_FRAME_SIZE);
        let total_frame_samples = frames_needed * AAC_FRAME_SIZE;
        let end_padding = (total_frame_samples - with_delay) as u32;

        // Verify frame alignment
        assert_eq!(total_frame_samples % AAC_FRAME_SIZE, 0);

        // Create delay and verify sample recovery
        let smpb = format!(
            " 00000000 {:08X} {:08X} {:016X}",
            encoder_delay, end_padding, original_samples
        );
        let delay = EncoderDelay::from_itun_smpb(&smpb).unwrap();
        assert_eq!(delay.actual_samples(total_frame_samples), original_samples);
    }

    /// Test MP3 frame alignment (1152 samples)
    #[test]
    fn test_mp3_frame_alignment() {
        const MP3_FRAME_SIZE: u64 = 1152;

        // Original samples
        let original_samples = 50000u64;
        let encoder_delay = 576u32; // LAME default

        // Calculate padding for frame alignment
        let with_delay = original_samples + encoder_delay as u64;
        let frames_needed = with_delay.div_ceil(MP3_FRAME_SIZE);
        let total_frame_samples = frames_needed * MP3_FRAME_SIZE;
        let end_padding = (total_frame_samples - with_delay) as u32;

        // Verify frame alignment
        assert_eq!(total_frame_samples % MP3_FRAME_SIZE, 0);

        // Create delay and verify
        let delay = EncoderDelay::from_lame(encoder_delay, end_padding);
        assert_eq!(delay.actual_samples(total_frame_samples), original_samples);
    }

    /// Test duration calculation precision at various sample rates
    #[test]
    fn test_duration_precision_all_sample_rates() {
        let sample_rates = [
            8000, 11025, 22050, 44100, 48000, 88200, 96000, 176400, 192000,
        ];
        let original_duration_secs = 180.0; // 3 minutes

        for &rate in &sample_rates {
            let original_samples = (rate as f64 * original_duration_secs) as u64;
            let delay = EncoderDelay::from_lame(576, 1152);
            let total = original_samples + 576 + 1152;

            let calculated_duration = delay.actual_duration(total, rate);
            let expected_duration = std::time::Duration::from_secs_f64(original_duration_secs);

            let error_ms = (calculated_duration.as_secs_f64() - expected_duration.as_secs_f64())
                .abs()
                * 1000.0;
            assert!(
                error_ms < 1.0,
                "Duration error at {}Hz: {:.3}ms",
                rate,
                error_ms
            );
        }
    }

    /// Test manual delay with typical correction values
    #[test]
    fn test_manual_delay_corrections() {
        // Sometimes manual correction is needed for files with missing metadata
        let corrections = [
            (576, 0, "Missing end padding"),
            (0, 1152, "Missing start padding"),
            (1024, 1024, "Generic lossy encoder"),
            (2112, 458, "AAC-like correction"),
        ];

        for (start, end, desc) in corrections {
            let delay = EncoderDelay::manual(start, end);
            assert_eq!(delay.source, DelaySource::Manual, "{}", desc);
            assert_eq!(delay.start_padding, start, "{}", desc);
            assert_eq!(delay.end_padding, end, "{}", desc);
        }
    }

    /// Test that DelayTrimmer handles zero total samples
    #[test]
    fn test_delay_trimmer_zero_samples() {
        let delay = EncoderDelay::from_lame(100, 100);
        let trimmer = DelayTrimmer::new(delay, 0);

        assert_eq!(trimmer.valid_samples(), 0);
        assert!(trimmer.at_end_padding()); // Nothing to play
    }

    /// Test start_samples_to_skip calculation
    #[test]
    fn test_start_samples_to_skip() {
        let delay = EncoderDelay::from_lame(100, 50);
        let mut trimmer = DelayTrimmer::new(delay, 1000);

        // At start, all 100 samples need to be skipped
        assert_eq!(trimmer.start_samples_to_skip(), 100);

        // After advancing 50, still 50 to skip
        trimmer.advance(50);
        assert_eq!(trimmer.start_samples_to_skip(), 50);

        // After advancing past start padding
        trimmer.advance(100);
        assert_eq!(trimmer.start_samples_to_skip(), 0);
    }
}
