//! Rubato resampler backend
//!
//! Fast, portable resampling using the rubato crate.
//! Good quality for real-time applications.

use super::{ResamplerImpl, ResamplingError, ResamplingQuality, Result};
use rubato::{
    FastFixedIn, FastFixedOut, Resampler as RubatoResamplerTrait, SincFixedIn, SincFixedOut,
    SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::collections::VecDeque;

/// Enum to hold different rubato resampler types
enum RubatoResamplerType {
    FastIn(FastFixedIn<f32>),
    FastOut(FastFixedOut<f32>),
    SincIn(SincFixedIn<f32>),
    SincOut(SincFixedOut<f32>),
}

/// Rubato-based resampler implementation
pub struct RubatoResampler {
    resampler: RubatoResamplerType,
    input_rate: u32,
    output_rate: u32,
    channels: usize,
    /// Buffer for accumulating input samples when they don't fill a complete chunk
    input_buffer: VecDeque<f32>,
    /// Chunk size the resampler was configured with
    chunk_size: usize,
}

impl RubatoResampler {
    /// Create a new rubato resampler
    pub fn new(
        input_rate: u32,
        output_rate: u32,
        channels: usize,
        quality: ResamplingQuality,
    ) -> Result<Self> {
        let ratio = output_rate as f64 / input_rate as f64;

        // Determine chunk size based on quality
        let chunk_size = match quality {
            ResamplingQuality::Fast => 1024,
            ResamplingQuality::Balanced => 1024,
            ResamplingQuality::High => 2048,
            ResamplingQuality::Maximum => 4096,
        };

        // Choose resampler based on quality and ratio
        let resampler = match quality {
            ResamplingQuality::Fast => {
                // Use FastFixed for fast quality
                if ratio >= 1.0 {
                    RubatoResamplerType::FastIn(
                        FastFixedIn::new(
                            ratio,
                            2.0, // max_resample_ratio_relative
                            rubato::PolynomialDegree::Linear,
                            chunk_size,
                            channels,
                        )
                        .map_err(|e| {
                            ResamplingError::InitializationFailed(format!(
                                "FastFixedIn creation failed: {}",
                                e
                            ))
                        })?,
                    )
                } else {
                    RubatoResamplerType::FastOut(
                        FastFixedOut::new(
                            ratio,
                            2.0, // max_resample_ratio_relative
                            rubato::PolynomialDegree::Linear,
                            chunk_size,
                            channels,
                        )
                        .map_err(|e| {
                            ResamplingError::InitializationFailed(format!(
                                "FastFixedOut creation failed: {}",
                                e
                            ))
                        })?,
                    )
                }
            }
            _ => {
                // Use Sinc for balanced/high/maximum quality
                let params = Self::quality_to_params(quality);

                if ratio >= 1.0 {
                    RubatoResamplerType::SincIn(
                        SincFixedIn::<f32>::new(
                            ratio, 2.0, // max_resample_ratio_relative
                            params, chunk_size, channels,
                        )
                        .map_err(|e| {
                            ResamplingError::InitializationFailed(format!(
                                "SincFixedIn creation failed: {}",
                                e
                            ))
                        })?,
                    )
                } else {
                    RubatoResamplerType::SincOut(
                        SincFixedOut::<f32>::new(
                            ratio, 2.0, // max_resample_ratio_relative
                            params, chunk_size, channels,
                        )
                        .map_err(|e| {
                            ResamplingError::InitializationFailed(format!(
                                "SincFixedOut creation failed: {}",
                                e
                            ))
                        })?,
                    )
                }
            }
        };

        Ok(Self {
            resampler,
            input_rate,
            output_rate,
            channels,
            input_buffer: VecDeque::new(),
            chunk_size,
        })
    }

    /// Convert quality preset to rubato parameters
    fn quality_to_params(quality: ResamplingQuality) -> SincInterpolationParameters {
        match quality {
            ResamplingQuality::Fast => SincInterpolationParameters {
                sinc_len: 64,
                f_cutoff: 0.9,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 128,
                window: WindowFunction::Blackman,
            },
            ResamplingQuality::Balanced => SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris,
            },
            ResamplingQuality::High => SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.99,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 512,
                window: WindowFunction::BlackmanHarris,
            },
            ResamplingQuality::Maximum => SincInterpolationParameters {
                sinc_len: 512,
                f_cutoff: 0.995,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 1024,
                window: WindowFunction::BlackmanHarris2,
            },
        }
    }

    /// Get expected input frame count for the next process call
    fn input_frames_next(&self) -> usize {
        match &self.resampler {
            RubatoResamplerType::FastIn(r) => r.input_frames_next(),
            RubatoResamplerType::FastOut(r) => r.input_frames_next(),
            RubatoResamplerType::SincIn(r) => r.input_frames_next(),
            RubatoResamplerType::SincOut(r) => r.input_frames_next(),
        }
    }

    /// Get maximum output frames for the configured chunk size
    fn output_frames_max(&self) -> usize {
        match &self.resampler {
            RubatoResamplerType::FastIn(r) => r.output_frames_max(),
            RubatoResamplerType::FastOut(r) => r.output_frames_max(),
            RubatoResamplerType::SincIn(r) => r.output_frames_max(),
            RubatoResamplerType::SincOut(r) => r.output_frames_max(),
        }
    }

    /// Get the latency in input frames
    pub fn latency(&self) -> usize {
        match &self.resampler {
            RubatoResamplerType::FastIn(r) => r.output_delay(),
            RubatoResamplerType::FastOut(r) => r.output_delay(),
            RubatoResamplerType::SincIn(r) => r.output_delay(),
            RubatoResamplerType::SincOut(r) => r.output_delay(),
        }
    }

    /// Deinterleave samples from [L, R, L, R, ...] to [[L, L, ...], [R, R, ...]]
    fn deinterleave(&self, interleaved: &[f32], frames: usize) -> Vec<Vec<f32>> {
        let mut channels = vec![Vec::with_capacity(frames); self.channels];

        for frame_idx in 0..frames {
            for ch in 0..self.channels {
                channels[ch].push(interleaved[frame_idx * self.channels + ch]);
            }
        }

        channels
    }

    /// Interleave samples from [[L, L, ...], [R, R, ...]] to [L, R, L, R, ...]
    fn interleave(&self, channels: Vec<Vec<f32>>) -> Vec<f32> {
        if channels.is_empty() {
            return Vec::new();
        }

        let frames = channels[0].len();
        let mut interleaved = Vec::with_capacity(frames * self.channels);

        for frame_idx in 0..frames {
            for ch in 0..self.channels {
                interleaved.push(channels[ch][frame_idx]);
            }
        }

        interleaved
    }

    /// Flush any remaining buffered samples using partial processing
    ///
    /// Call this at the end of a stream to retrieve any samples that were
    /// buffered but not yet processed (because they didn't fill a complete chunk).
    pub fn flush(&mut self) -> Result<Vec<f32>> {
        // 1:1 passthrough - just drain the buffer
        if self.input_rate == self.output_rate {
            return Ok(self.input_buffer.drain(..).collect());
        }

        if self.input_buffer.is_empty() {
            return Ok(Vec::new());
        }

        // Process remaining buffered samples using process_partial
        let remaining_samples: Vec<f32> = self.input_buffer.drain(..).collect();
        let frames = remaining_samples.len() / self.channels;

        if frames == 0 {
            return Ok(Vec::new());
        }

        let input_channels = self.deinterleave(&remaining_samples, frames);

        let output_channels = match &mut self.resampler {
            RubatoResamplerType::FastIn(r) => r
                .process_partial(Some(&input_channels), None)
                .map_err(|e| {
                    ResamplingError::ProcessingFailed(format!("FastIn flush failed: {}", e))
                })?,
            RubatoResamplerType::FastOut(r) => r
                .process_partial(Some(&input_channels), None)
                .map_err(|e| {
                    ResamplingError::ProcessingFailed(format!("FastOut flush failed: {}", e))
                })?,
            RubatoResamplerType::SincIn(r) => r
                .process_partial(Some(&input_channels), None)
                .map_err(|e| {
                    ResamplingError::ProcessingFailed(format!("SincIn flush failed: {}", e))
                })?,
            RubatoResamplerType::SincOut(r) => r
                .process_partial(Some(&input_channels), None)
                .map_err(|e| {
                    ResamplingError::ProcessingFailed(format!("SincOut flush failed: {}", e))
                })?,
        };

        Ok(self.interleave(output_channels))
    }
}

impl ResamplerImpl for RubatoResampler {
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

        // 1:1 passthrough optimization - avoid resampling overhead when rates match
        if self.input_rate == self.output_rate {
            return Ok(input.to_vec());
        }

        // Add all input to the buffer
        self.input_buffer.extend(input.iter().copied());

        let mut output = Vec::new();

        // Process complete chunks only - this fixes the accumulated output ratio bug
        // by only calling process() when we have exactly the expected number of input frames
        loop {
            let needed_frames = self.input_frames_next();
            let needed_samples = needed_frames * self.channels;

            if self.input_buffer.len() < needed_samples {
                // Not enough samples for a complete chunk, wait for more input
                break;
            }

            // Extract exactly the needed samples from the buffer
            let chunk: Vec<f32> = self.input_buffer.drain(..needed_samples).collect();

            // Deinterleave chunk
            let input_channels = self.deinterleave(&chunk, needed_frames);

            // Process the complete chunk (always use process(), never process_partial for normal operation)
            let output_channels = match &mut self.resampler {
                RubatoResamplerType::FastIn(r) => {
                    r.process(&input_channels, None).map_err(|e| {
                        ResamplingError::ProcessingFailed(format!(
                            "FastIn resampling failed: {}",
                            e
                        ))
                    })?
                }
                RubatoResamplerType::FastOut(r) => {
                    r.process(&input_channels, None).map_err(|e| {
                        ResamplingError::ProcessingFailed(format!(
                            "FastOut resampling failed: {}",
                            e
                        ))
                    })?
                }
                RubatoResamplerType::SincIn(r) => {
                    r.process(&input_channels, None).map_err(|e| {
                        ResamplingError::ProcessingFailed(format!(
                            "SincIn resampling failed: {}",
                            e
                        ))
                    })?
                }
                RubatoResamplerType::SincOut(r) => {
                    r.process(&input_channels, None).map_err(|e| {
                        ResamplingError::ProcessingFailed(format!(
                            "SincOut resampling failed: {}",
                            e
                        ))
                    })?
                }
            };

            // Interleave and append to output
            output.extend(self.interleave(output_channels));
        }

        Ok(output)
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
        // Clear the input buffer to remove any leftover samples
        self.input_buffer.clear();

        // Reset the underlying resampler's filter state
        match &mut self.resampler {
            RubatoResamplerType::FastIn(r) => r.reset(),
            RubatoResamplerType::FastOut(r) => r.reset(),
            RubatoResamplerType::SincIn(r) => r.reset(),
            RubatoResamplerType::SincOut(r) => r.reset(),
        }
    }

    fn latency(&self) -> usize {
        self.latency()
    }

    fn flush(&mut self) -> Result<Vec<f32>> {
        self.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rubato_creation() {
        let resampler = RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Balanced).unwrap();
        assert_eq!(resampler.input_rate(), 44100);
        assert_eq!(resampler.output_rate(), 96000);
        assert_eq!(resampler.channels(), 2);
    }

    #[test]
    fn test_deinterleave_interleave() {
        let resampler = RubatoResampler::new(44100, 48000, 2, ResamplingQuality::Fast).unwrap();

        // Test data: [L0, R0, L1, R1, ...]
        let interleaved = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let deinterleaved = resampler.deinterleave(&interleaved, 3);
        assert_eq!(deinterleaved.len(), 2);
        assert_eq!(deinterleaved[0], vec![1.0, 3.0, 5.0]); // Left channel
        assert_eq!(deinterleaved[1], vec![2.0, 4.0, 6.0]); // Right channel

        let reinterleaved = resampler.interleave(deinterleaved);
        assert_eq!(reinterleaved, interleaved);
    }

    #[test]
    fn test_process_empty() {
        let mut resampler = RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        let output = resampler.process(&[]).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_process_invalid_size() {
        let mut resampler = RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        // Odd number of samples for stereo (should fail)
        let result = resampler.process(&[1.0, 2.0, 3.0]);
        assert!(result.is_err());
    }
}
