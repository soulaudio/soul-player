//! High-Quality Audio Resampling
//!
//! This module provides professional-grade sample rate conversion for audiophile playback.
//!
//! ## Features
//!
//! - **Multiple backends**:
//!   - `r8brain-rs`: Highest quality for critical listening (default)
//!   - `rubato`: Fast, portable fallback
//! - **Quality presets**: Fast, Balanced, High, Maximum
//! - **Arbitrary sample rates**: 44.1kHz → 96kHz, 192kHz, etc.
//! - **Real-time performance**: Optimized for live playback
//!
//! ## Example
//!
//! ```rust
//! use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
//!
//! // Create high-quality resampler (44.1kHz → 96kHz)
//! // Auto uses r8brain if available (requires r8brain feature), otherwise rubato
//! let mut resampler = Resampler::new(
//!     ResamplerBackend::Auto,
//!     44100,
//!     96000,
//!     2, // stereo
//!     ResamplingQuality::High,
//! ).unwrap();
//!
//! // Process audio
//! let input = vec![0.0; 2048]; // Stereo samples
//! let output = resampler.process(&input).unwrap();
//! ```

#[cfg(feature = "r8brain")]
mod r8brain;
mod rubato_backend;

use thiserror::Error;

#[cfg(feature = "r8brain")]
pub use r8brain::R8BrainResampler;
pub use rubato_backend::RubatoResampler;

/// Resampling errors
#[derive(Error, Debug)]
pub enum ResamplingError {
    #[error("Invalid sample rate: {0} Hz (must be > 0 and < 1MHz)")]
    InvalidSampleRate(u32),

    #[error("Invalid channel count: {0} (must be 1-8)")]
    InvalidChannelCount(usize),

    #[error("Input buffer size mismatch: expected {expected}, got {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },

    #[error("Resampler initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
}

pub type Result<T> = std::result::Result<T, ResamplingError>;

/// Resampling quality presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResamplingQuality {
    /// Fast - Low CPU, good for real-time streaming
    /// - Passband: 90% of Nyquist
    /// - Stopband attenuation: 60 dB
    Fast,

    /// Balanced - Moderate CPU, good quality
    /// - Passband: 95% of Nyquist
    /// - Stopband attenuation: 100 dB
    Balanced,

    /// High - Higher CPU, excellent quality
    /// - Passband: 99% of Nyquist
    /// - Stopband attenuation: 140 dB
    High,

    /// Maximum - Highest CPU, audiophile quality
    /// - Passband: 99.5% of Nyquist
    /// - Stopband attenuation: 180 dB
    Maximum,
}

impl ResamplingQuality {
    /// Get transition band width (0.0 - 1.0, normalized to Nyquist)
    pub fn transition_band(&self) -> f64 {
        match self {
            Self::Fast => 0.10,     // 10% transition band
            Self::Balanced => 0.05, // 5% transition band
            Self::High => 0.01,     // 1% transition band
            Self::Maximum => 0.005, // 0.5% transition band
        }
    }

    /// Get stopband attenuation in dB
    pub fn stopband_attenuation_db(&self) -> f64 {
        match self {
            Self::Fast => 60.0,
            Self::Balanced => 100.0,
            Self::High => 140.0,
            Self::Maximum => 180.0,
        }
    }
}

/// Resampler backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResamplerBackend {
    /// r8brain-rs: Highest quality, audiophile-grade
    R8Brain,

    /// rubato: Fast, portable, good quality
    Rubato,

    /// Auto: Choose best available (r8brain if available, else rubato)
    Auto,
}

/// Trait for resampler implementations
pub trait ResamplerImpl: Send {
    /// Process interleaved audio samples
    ///
    /// # Arguments
    /// - `input`: Interleaved input samples (e.g., [L, R, L, R, ...])
    ///
    /// # Returns
    /// Interleaved output samples at target sample rate
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>>;

    /// Get input sample rate
    fn input_rate(&self) -> u32;

    /// Get output sample rate
    fn output_rate(&self) -> u32;

    /// Get channel count
    fn channels(&self) -> usize;

    /// Reset internal state
    fn reset(&mut self);
}

/// High-level resampler interface
pub struct Resampler {
    backend: Box<dyn ResamplerImpl>,
}

impl Resampler {
    /// Create a new resampler
    ///
    /// # Arguments
    /// - `backend`: Resampler backend to use
    /// - `input_rate`: Input sample rate (Hz)
    /// - `output_rate`: Output sample rate (Hz)
    /// - `channels`: Number of channels (1-8)
    /// - `quality`: Quality preset
    pub fn new(
        backend: ResamplerBackend,
        input_rate: u32,
        output_rate: u32,
        channels: usize,
        quality: ResamplingQuality,
    ) -> Result<Self> {
        // Validate inputs
        if input_rate == 0 || input_rate > 1_000_000 {
            return Err(ResamplingError::InvalidSampleRate(input_rate));
        }
        if output_rate == 0 || output_rate > 1_000_000 {
            return Err(ResamplingError::InvalidSampleRate(output_rate));
        }
        if channels == 0 || channels > 8 {
            return Err(ResamplingError::InvalidChannelCount(channels));
        }

        let backend_impl: Box<dyn ResamplerImpl> = match backend {
            #[cfg(feature = "r8brain")]
            ResamplerBackend::R8Brain => {
                Box::new(R8BrainResampler::new(
                    input_rate,
                    output_rate,
                    channels,
                    quality,
                )?)
            }
            #[cfg(not(feature = "r8brain"))]
            ResamplerBackend::R8Brain => {
                return Err(ResamplingError::InitializationFailed(
                    "r8brain backend not available (feature not enabled)".to_string(),
                ));
            }
            ResamplerBackend::Rubato => Box::new(RubatoResampler::new(
                input_rate,
                output_rate,
                channels,
                quality,
            )?),
            ResamplerBackend::Auto => {
                // Auto: Use r8brain if available, otherwise rubato
                #[cfg(feature = "r8brain")]
                {
                    Box::new(R8BrainResampler::new(
                        input_rate,
                        output_rate,
                        channels,
                        quality,
                    )?)
                }
                #[cfg(not(feature = "r8brain"))]
                {
                    Box::new(RubatoResampler::new(
                        input_rate,
                        output_rate,
                        channels,
                        quality,
                    )?)
                }
            }
        };

        Ok(Self {
            backend: backend_impl,
        })
    }

    /// Process interleaved audio samples
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        self.backend.process(input)
    }

    /// Get input sample rate
    pub fn input_rate(&self) -> u32 {
        self.backend.input_rate()
    }

    /// Get output sample rate
    pub fn output_rate(&self) -> u32 {
        self.backend.output_rate()
    }

    /// Get channel count
    pub fn channels(&self) -> usize {
        self.backend.channels()
    }

    /// Reset internal state
    pub fn reset(&mut self) {
        self.backend.reset();
    }

    /// Calculate expected output size for given input size
    ///
    /// Useful for pre-allocating buffers
    pub fn calculate_output_size(&self, input_samples: usize) -> usize {
        let ratio = self.output_rate() as f64 / self.input_rate() as f64;
        (input_samples as f64 * ratio).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_presets() {
        assert!(ResamplingQuality::Fast.transition_band() > ResamplingQuality::Balanced.transition_band());
        assert!(ResamplingQuality::Balanced.transition_band() > ResamplingQuality::High.transition_band());
        assert!(ResamplingQuality::High.transition_band() > ResamplingQuality::Maximum.transition_band());

        assert!(ResamplingQuality::Fast.stopband_attenuation_db() < ResamplingQuality::Maximum.stopband_attenuation_db());
    }

    #[test]
    fn test_invalid_sample_rates() {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            0,
            96000,
            2,
            ResamplingQuality::High,
        );
        assert!(matches!(result, Err(ResamplingError::InvalidSampleRate(0))));

        let result = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            2_000_000,
            2,
            ResamplingQuality::High,
        );
        assert!(matches!(
            result,
            Err(ResamplingError::InvalidSampleRate(2_000_000))
        ));
    }

    #[test]
    fn test_invalid_channels() {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            0,
            ResamplingQuality::High,
        );
        assert!(matches!(
            result,
            Err(ResamplingError::InvalidChannelCount(0))
        ));

        let result = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            10,
            ResamplingQuality::High,
        );
        assert!(matches!(
            result,
            Err(ResamplingError::InvalidChannelCount(10))
        ));
    }

    #[test]
    fn test_output_size_calculation() {
        // 44.1kHz → 96kHz upsampling
        let resampler = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            2,
            ResamplingQuality::Balanced,
        )
        .unwrap();

        let input_samples = 2048; // stereo interleaved
        let expected_output = (2048.0f64 * (96000.0f64 / 44100.0f64)).ceil() as usize;
        assert_eq!(resampler.calculate_output_size(input_samples), expected_output);
    }
}
