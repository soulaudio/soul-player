//! DoP (DSD over PCM) encoding and decoding
//!
//! DoP is a method of transporting DSD audio over PCM interfaces by
//! packing DSD bits into the lower 24 bits of PCM samples with a
//! marker byte in the upper 8 bits.
//!
//! DoP Format (32-bit):
//! - Bits 31-24: Marker (0x05 or 0xFA, alternating)
//! - Bits 23-16: DSD byte 1
//! - Bits 15-8:  DSD byte 2 (or padding)
//! - Bits 7-0:   DSD byte 3 (or padding)
//!
//! For DoP64 (DSD64 over PCM):
//! - Uses 176.4 kHz sample rate (4x 44.1 kHz)
//! - Each 32-bit sample contains 16 DSD bits (2 bytes) per channel
//!
//! For DoP128 (DSD128 over PCM):
//! - Uses 352.8 kHz sample rate (8x 44.1 kHz)
//! - Each 32-bit sample contains 16 DSD bits (2 bytes) per channel

/// DoP marker bytes (alternating to detect errors)
pub const DOP_MARKER_A: u8 = 0x05;
pub const DOP_MARKER_B: u8 = 0xFA;

/// DoP format information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoP {
    /// DSD sample rate
    pub dsd_rate_hz: u32,

    /// Required PCM sample rate for transport
    pub pcm_rate_hz: u32,

    /// Bits per sample (always 24 for DoP payload + 8 for marker)
    pub bits_per_sample: u8,
}

impl DoP {
    /// DoP for DSD64 (2.8224 MHz) - transported at 176.4 kHz
    pub const DSD64: DoP = DoP {
        dsd_rate_hz: 2_822_400,
        pcm_rate_hz: 176_400,
        bits_per_sample: 32,
    };

    /// DoP for DSD128 (5.6448 MHz) - transported at 352.8 kHz
    pub const DSD128: DoP = DoP {
        dsd_rate_hz: 5_644_800,
        pcm_rate_hz: 352_800,
        bits_per_sample: 32,
    };

    /// DoP for DSD256 (11.2896 MHz) - transported at 705.6 kHz
    pub const DSD256: DoP = DoP {
        dsd_rate_hz: 11_289_600,
        pcm_rate_hz: 705_600,
        bits_per_sample: 32,
    };
}

/// DoP encoder - converts packed DSD bytes to DoP PCM format
pub struct DopEncoder {
    /// Current marker state (alternates between A and B)
    marker_state: bool,

    /// Number of channels
    channels: usize,
}

impl DopEncoder {
    /// Create a new DoP encoder
    pub fn new(channels: usize) -> Self {
        Self {
            marker_state: false,
            channels,
        }
    }

    /// Get the current marker byte
    fn current_marker(&self) -> u8 {
        if self.marker_state {
            DOP_MARKER_B
        } else {
            DOP_MARKER_A
        }
    }

    /// Encode packed DSD bytes to DoP format (32-bit integers)
    ///
    /// # Arguments
    /// * `dsd_input` - Packed DSD bytes (interleaved by channel)
    ///
    /// # Returns
    /// DoP samples as 32-bit integers (marker in upper byte, DSD in lower 24 bits)
    pub fn encode(&mut self, dsd_input: &[u8]) -> Vec<i32> {
        // Each DoP sample contains 2 DSD bytes (16 bits) per channel
        // So we process 2 bytes at a time per channel
        let bytes_per_sample = 2;
        let bytes_per_frame = bytes_per_sample * self.channels;
        let num_frames = dsd_input.len() / bytes_per_frame;

        let mut output = Vec::with_capacity(num_frames * self.channels);

        for frame_idx in 0..num_frames {
            let frame_offset = frame_idx * bytes_per_frame;

            for ch in 0..self.channels {
                let byte_offset = frame_offset + ch * bytes_per_sample;

                if byte_offset + 1 < dsd_input.len() {
                    let marker = self.current_marker();
                    let dsd_byte1 = dsd_input[byte_offset];
                    let dsd_byte2 = dsd_input[byte_offset + 1];

                    // Pack into 32-bit DoP format
                    // Marker in bits 31-24, DSD bytes in bits 23-8, bits 7-0 unused
                    let dop_sample = ((marker as i32) << 24)
                        | ((dsd_byte1 as i32) << 16)
                        | ((dsd_byte2 as i32) << 8);

                    output.push(dop_sample);
                }
            }

            // Alternate marker for next frame
            self.marker_state = !self.marker_state;
        }

        output
    }

    /// Encode packed DSD bytes to DoP format as f32 samples
    ///
    /// This is useful for audio APIs that expect float samples.
    /// The DoP data is encoded in the significand bits.
    pub fn encode_f32(&mut self, dsd_input: &[u8]) -> Vec<f32> {
        let i32_output = self.encode(dsd_input);

        // Convert to normalized float (-1.0 to 1.0)
        // Note: This loses precision in the marker, but many DACs
        // will recognize DoP from the pattern
        i32_output
            .iter()
            .map(|&sample| sample as f32 / i32::MAX as f32)
            .collect()
    }

    /// Reset encoder state
    pub fn reset(&mut self) {
        self.marker_state = false;
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.channels
    }
}

impl Default for DopEncoder {
    fn default() -> Self {
        Self::new(2)
    }
}

/// DoP decoder - extracts DSD bytes from DoP PCM format
pub struct DopDecoder {
    /// Number of channels
    channels: usize,

    /// Expected marker state for error detection
    expected_marker: bool,
}

impl DopDecoder {
    /// Create a new DoP decoder
    pub fn new(channels: usize) -> Self {
        Self {
            channels,
            expected_marker: false,
        }
    }

    /// Check if a sample appears to be DoP encoded
    pub fn is_dop_sample(sample: i32) -> bool {
        let marker = ((sample >> 24) & 0xFF) as u8;
        marker == DOP_MARKER_A || marker == DOP_MARKER_B
    }

    /// Decode DoP samples back to packed DSD bytes
    ///
    /// # Arguments
    /// * `dop_input` - DoP samples as 32-bit integers
    ///
    /// # Returns
    /// Some(packed DSD bytes) if valid DoP, None if invalid markers
    pub fn decode(&mut self, dop_input: &[i32]) -> Option<Vec<u8>> {
        let num_samples = dop_input.len();
        let num_frames = num_samples / self.channels;

        let bytes_per_sample = 2;
        let mut output = Vec::with_capacity(num_frames * self.channels * bytes_per_sample);

        for frame_idx in 0..num_frames {
            let frame_offset = frame_idx * self.channels;

            for ch in 0..self.channels {
                let sample = dop_input[frame_offset + ch];

                // Extract marker and verify
                let marker = ((sample >> 24) & 0xFF) as u8;
                let expected = if self.expected_marker {
                    DOP_MARKER_B
                } else {
                    DOP_MARKER_A
                };

                if marker != expected && marker != DOP_MARKER_A && marker != DOP_MARKER_B {
                    // Not valid DoP
                    return None;
                }

                // Extract DSD bytes
                let dsd_byte1 = ((sample >> 16) & 0xFF) as u8;
                let dsd_byte2 = ((sample >> 8) & 0xFF) as u8;

                output.push(dsd_byte1);
                output.push(dsd_byte2);
            }

            self.expected_marker = !self.expected_marker;
        }

        Some(output)
    }

    /// Reset decoder state
    pub fn reset(&mut self) {
        self.expected_marker = false;
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.channels
    }
}

impl Default for DopDecoder {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dop_constants() {
        assert_eq!(DoP::DSD64.dsd_rate_hz, 2_822_400);
        assert_eq!(DoP::DSD64.pcm_rate_hz, 176_400);

        assert_eq!(DoP::DSD128.dsd_rate_hz, 5_644_800);
        assert_eq!(DoP::DSD128.pcm_rate_hz, 352_800);
    }

    #[test]
    fn test_encoder_creation() {
        let encoder = DopEncoder::new(2);
        assert_eq!(encoder.channels(), 2);
    }

    #[test]
    fn test_encode_basic() {
        let mut encoder = DopEncoder::new(1); // Mono for simplicity

        // 4 bytes of DSD data (2 DoP samples worth)
        let dsd_input = vec![0xAA, 0x55, 0xBB, 0x66];
        let output = encoder.encode(&dsd_input);

        assert_eq!(output.len(), 2);

        // First sample should have marker A
        let marker1 = ((output[0] >> 24) & 0xFF) as u8;
        assert_eq!(marker1, DOP_MARKER_A);

        // Second sample should have marker B
        let marker2 = ((output[1] >> 24) & 0xFF) as u8;
        assert_eq!(marker2, DOP_MARKER_B);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut encoder = DopEncoder::new(2);
        let mut decoder = DopDecoder::new(2);

        // Original DSD data (stereo, 4 frames = 16 bytes)
        let original: Vec<u8> = (0..16).collect();

        // Encode to DoP
        let dop = encoder.encode(&original);

        // Decode back
        let decoded = decoder.decode(&dop).expect("Should decode successfully");

        // Should match original
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_is_dop_sample() {
        // Valid DoP markers
        let sample_a = (DOP_MARKER_A as i32) << 24;
        let sample_b = (DOP_MARKER_B as i32) << 24;

        assert!(DopDecoder::is_dop_sample(sample_a));
        assert!(DopDecoder::is_dop_sample(sample_b));

        // Invalid marker
        let sample_invalid = 0x12 << 24;
        assert!(!DopDecoder::is_dop_sample(sample_invalid));
    }

    #[test]
    fn test_decode_invalid() {
        let mut decoder = DopDecoder::new(1);

        // Invalid DoP data (wrong markers)
        let invalid = vec![0x12345678_i32];
        let result = decoder.decode(&invalid);

        assert!(result.is_none());
    }

    #[test]
    fn test_encoder_reset() {
        let mut encoder = DopEncoder::new(1);

        // Encode something
        let dsd = vec![0xAA, 0x55];
        encoder.encode(&dsd);

        // Reset
        encoder.reset();

        // Should start with marker A again
        let output = encoder.encode(&dsd);
        let marker = ((output[0] >> 24) & 0xFF) as u8;
        assert_eq!(marker, DOP_MARKER_A);
    }

    #[test]
    fn test_stereo_encoding() {
        let mut encoder = DopEncoder::new(2);

        // Stereo DSD data: L1, L2, R1, R2, L3, L4, R3, R4 (2 frames)
        let dsd_input = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let output = encoder.encode(&dsd_input);

        // Should produce 4 samples (2 frames * 2 channels)
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_encode_f32() {
        let mut encoder = DopEncoder::new(1);

        let dsd = vec![0xAA, 0x55];
        let f32_output = encoder.encode_f32(&dsd);

        assert_eq!(f32_output.len(), 1);
        // Value should be non-zero
        assert!(f32_output[0].abs() > 0.0);
    }
}
