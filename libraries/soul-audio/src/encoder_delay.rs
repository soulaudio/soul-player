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
        let encoder_delay =
            ((header_bytes[0] as u32) << 4) | ((header_bytes[1] as u32) >> 4);
        let end_padding =
            (((header_bytes[1] as u32) & 0x0F) << 8) | (header_bytes[2] as u32);

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
        let valid_end = self.total_samples.saturating_sub(self.delay.end_padding as u64);
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
        let valid_end = self.total_samples.saturating_sub(self.delay.end_padding as u64);
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
        if self.samples_read <= self.delay.start_padding as u64 {
            0
        } else {
            self.samples_read - self.delay.start_padding as u64
        }
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
        assert_eq!(delay.start_padding, 0x840);  // 2112
        assert_eq!(delay.end_padding, 0xAAC);    // 2732
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
        assert_eq!(raw_pos, 600);  // 500 + 100 start padding
        assert_eq!(trimmer.position(), 500);
    }

    #[test]
    fn test_delay_trimmer_valid_samples() {
        let delay = EncoderDelay::from_lame(100, 50);
        let trimmer = DelayTrimmer::new(delay, 1000);
        assert_eq!(trimmer.valid_samples(), 850);  // 1000 - 100 - 50
    }

    #[test]
    fn test_actual_duration() {
        let delay = EncoderDelay::from_lame(576, 1152);
        let duration = delay.actual_duration(44100, 44100);
        // 44100 - 576 - 1152 = 42372 samples = ~0.961 seconds
        assert!((duration.as_secs_f64() - 0.961).abs() < 0.01);
    }
}
