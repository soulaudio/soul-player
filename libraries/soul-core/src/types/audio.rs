/// Audio-related types
use serde::{Deserialize, Serialize};

/// Sample rate in Hz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SampleRate(pub u32);

impl SampleRate {
    /// Common sample rates
    pub const CD_QUALITY: Self = Self(44_100);
    pub const DVD_QUALITY: Self = Self(48_000);
    pub const HIGH_RES_88: Self = Self(88_200);
    pub const HIGH_RES_96: Self = Self(96_000);
    pub const HIGH_RES_176: Self = Self(176_400);
    pub const HIGH_RES_192: Self = Self(192_000);

    /// Create a new sample rate
    #[must_use]
    pub fn new(hz: u32) -> Self {
        Self(hz)
    }

    /// Get the sample rate as Hz
    pub fn as_hz(&self) -> u32 {
        self.0
    }
}

/// Audio format information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFormat {
    /// Sample rate
    pub sample_rate: SampleRate,

    /// Number of channels (1 = mono, 2 = stereo, etc.)
    pub channels: u16,

    /// Bits per sample
    pub bits_per_sample: u16,
}

impl AudioFormat {
    /// Create a new audio format
    pub fn new(sample_rate: SampleRate, channels: u16, bits_per_sample: u16) -> Self {
        Self {
            sample_rate,
            channels,
            bits_per_sample,
        }
    }

    /// Create CD quality stereo format (44.1kHz, 16-bit, stereo)
    pub fn cd_quality() -> Self {
        Self {
            sample_rate: SampleRate::CD_QUALITY,
            channels: 2,
            bits_per_sample: 16,
        }
    }

    /// Calculate the byte rate (bytes per second)
    pub fn byte_rate(&self) -> u32 {
        self.sample_rate.as_hz() * u32::from(self.channels) * u32::from(self.bits_per_sample) / 8
    }
}

/// Audio buffer containing decoded samples
///
/// Samples are stored as f32 in the range [-1.0, 1.0]
/// Interleaved format: [L, R, L, R, ...] for stereo
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Audio samples (f32, interleaved)
    pub samples: Vec<f32>,

    /// Audio format information
    pub format: AudioFormat,
}

impl AudioBuffer {
    /// Create a new audio buffer
    pub fn new(samples: Vec<f32>, format: AudioFormat) -> Self {
        Self { samples, format }
    }

    /// Create an empty audio buffer with a given capacity
    pub fn with_capacity(capacity: usize, format: AudioFormat) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            format,
        }
    }

    /// Get the number of frames (samples per channel)
    pub fn frames(&self) -> usize {
        self.samples.len() / self.format.channels as usize
    }

    /// Get the duration in seconds
    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / self.format.sample_rate.as_hz() as f64
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Get the length in samples
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_common_values() {
        assert_eq!(SampleRate::CD_QUALITY.as_hz(), 44_100);
        assert_eq!(SampleRate::DVD_QUALITY.as_hz(), 48_000);
    }

    #[test]
    fn audio_format_byte_rate() {
        let format = AudioFormat::cd_quality();
        // 44100 Hz * 2 channels * 16 bits / 8 = 176,400 bytes/sec
        assert_eq!(format.byte_rate(), 176_400);
    }

    #[test]
    fn audio_buffer_frames_calculation() {
        let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 16);
        // 8 samples with 2 channels = 4 frames
        let buffer = AudioBuffer::new(vec![0.0; 8], format);
        assert_eq!(buffer.frames(), 4);
    }

    #[test]
    fn audio_buffer_duration() {
        let format = AudioFormat::new(SampleRate::new(44_100), 2, 16);
        // 88200 samples with 2 channels = 44100 frames = 1 second
        let buffer = AudioBuffer::new(vec![0.0; 88_200], format);
        assert!((buffer.duration_secs() - 1.0).abs() < 0.01);
    }
}
