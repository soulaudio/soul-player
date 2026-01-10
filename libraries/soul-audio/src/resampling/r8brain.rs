//! r8brain resampler backend
//!
//! Highest quality resampling using the r8brain-rs crate.
//! Professional-grade audiophile quality with minimal artifacts.

use super::{ResamplerImpl, ResamplingError, ResamplingQuality, Result};
use r8brain_rs::{ConversionQuality, Resampler as R8BrainResamplerCore};

/// r8brain-based resampler implementation
pub struct R8BrainResampler {
    resamplers: Vec<R8BrainResamplerCore>,
    input_rate: u32,
    output_rate: u32,
    channels: usize,
}

impl R8BrainResampler {
    /// Create a new r8brain resampler
    pub fn new(
        input_rate: u32,
        output_rate: u32,
        channels: usize,
        quality: ResamplingQuality,
    ) -> Result<Self> {
        let r8brain_quality = Self::quality_to_r8brain(quality);

        // Create one resampler per channel
        let mut resamplers = Vec::with_capacity(channels);
        for _ in 0..channels {
            let resampler = R8BrainResamplerCore::new(
                input_rate as f64,
                output_rate as f64,
                1024, // max_in_frames - reasonable chunk size
                r8brain_quality,
            )
            .map_err(|e| {
                ResamplingError::InitializationFailed(format!("r8brain creation failed: {:?}", e))
            })?;

            resamplers.push(resampler);
        }

        Ok(Self {
            resamplers,
            input_rate,
            output_rate,
            channels,
        })
    }

    /// Convert quality preset to r8brain quality
    fn quality_to_r8brain(quality: ResamplingQuality) -> ConversionQuality {
        match quality {
            ResamplingQuality::Fast => ConversionQuality::Fast,
            ResamplingQuality::Balanced => ConversionQuality::Medium,
            ResamplingQuality::High => ConversionQuality::High,
            ResamplingQuality::Maximum => ConversionQuality::VeryHigh,
        }
    }

    /// Deinterleave samples from [L, R, L, R, ...] to [[L, L, ...], [R, R, ...]]
    fn deinterleave(&self, interleaved: &[f32]) -> Vec<Vec<f64>> {
        let frames = interleaved.len() / self.channels;
        let mut channels = vec![Vec::with_capacity(frames); self.channels];

        for frame_idx in 0..frames {
            for ch in 0..self.channels {
                // Convert f32 to f64 for r8brain
                channels[ch].push(interleaved[frame_idx * self.channels + ch] as f64);
            }
        }

        channels
    }

    /// Interleave samples from [[L, L, ...], [R, R, ...]] to [L, R, L, R, ...]
    fn interleave(&self, channels: Vec<Vec<f64>>) -> Vec<f32> {
        if channels.is_empty() {
            return Vec::new();
        }

        let frames = channels[0].len();
        let mut interleaved = Vec::with_capacity(frames * self.channels);

        for frame_idx in 0..frames {
            for ch in 0..self.channels {
                // Convert f64 back to f32
                interleaved.push(channels[ch][frame_idx] as f32);
            }
        }

        interleaved
    }
}

impl ResamplerImpl for R8BrainResampler {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        if input.len() % self.channels != 0 {
            return Err(ResamplingError::ProcessingFailed(format!(
                "Input buffer size {} is not a multiple of channel count {}",
                input.len(),
                self.channels
            )));
        }

        // Deinterleave input
        let input_channels = self.deinterleave(input);

        // Process each channel independently
        let mut output_channels = Vec::with_capacity(self.channels);

        for (ch_idx, channel_data) in input_channels.iter().enumerate() {
            let output = self.resamplers[ch_idx]
                .process(channel_data)
                .map_err(|e| {
                    ResamplingError::ProcessingFailed(format!(
                        "r8brain processing failed on channel {}: {:?}",
                        ch_idx, e
                    ))
                })?;

            output_channels.push(output);
        }

        // Interleave output
        Ok(self.interleave(output_channels))
    }

    fn input_rate(&self) -> u32 {
        self.input_rate
    }

    fn output_rate(&self) -> u32 {
        self.output_rate
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn reset(&mut self) {
        for resampler in &mut self.resamplers {
            resampler.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r8brain_creation() {
        let resampler =
            R8BrainResampler::new(44100, 96000, 2, ResamplingQuality::High).unwrap();
        assert_eq!(resampler.input_rate(), 44100);
        assert_eq!(resampler.output_rate(), 96000);
        assert_eq!(resampler.channels(), 2);
    }

    #[test]
    fn test_quality_mapping() {
        assert_eq!(
            R8BrainResampler::quality_to_r8brain(ResamplingQuality::Fast),
            ConversionQuality::Fast
        );
        assert_eq!(
            R8BrainResampler::quality_to_r8brain(ResamplingQuality::Balanced),
            ConversionQuality::Medium
        );
        assert_eq!(
            R8BrainResampler::quality_to_r8brain(ResamplingQuality::High),
            ConversionQuality::High
        );
        assert_eq!(
            R8BrainResampler::quality_to_r8brain(ResamplingQuality::Maximum),
            ConversionQuality::VeryHigh
        );
    }

    #[test]
    fn test_deinterleave_interleave() {
        let resampler =
            R8BrainResampler::new(44100, 48000, 2, ResamplingQuality::Fast).unwrap();

        // Test data: [L0, R0, L1, R1, ...]
        let interleaved = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let deinterleaved = resampler.deinterleave(&interleaved);
        assert_eq!(deinterleaved.len(), 2);
        assert_eq!(deinterleaved[0], vec![1.0, 3.0, 5.0]); // Left channel
        assert_eq!(deinterleaved[1], vec![2.0, 4.0, 6.0]); // Right channel

        let reinterleaved = resampler.interleave(deinterleaved);

        // Compare with tolerance for f32/f64 conversion
        for (a, b) in reinterleaved.iter().zip(interleaved.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_process_empty() {
        let mut resampler =
            R8BrainResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        let output = resampler.process(&[]).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_process_invalid_size() {
        let mut resampler =
            R8BrainResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        // Odd number of samples for stereo (should fail)
        let result = resampler.process(&[1.0, 2.0, 3.0]);
        assert!(result.is_err());
    }
}
