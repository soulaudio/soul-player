//! Rubato resampler backend
//!
//! Fast, portable resampling using the rubato crate.
//! Good quality for real-time applications.

use super::{ResamplerImpl, ResamplingError, ResamplingQuality, Result};
use rubato::{
    FastFixedIn, FastFixedOut, Resampler as RubatoResamplerTrait,
    SincFixedIn, SincFixedOut, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

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

        // Choose resampler based on quality and ratio
        let resampler = match quality {
            ResamplingQuality::Fast => {
                // Use FastFixed for fast quality
                if ratio >= 1.0 {
                    RubatoResamplerType::FastIn(
                        FastFixedIn::new(
                            ratio,
                            2.0,      // max_resample_ratio_relative
                            rubato::PolynomialDegree::Linear,
                            1024,     // chunk_size
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
                            2.0,      // max_resample_ratio_relative
                            rubato::PolynomialDegree::Linear,
                            1024,     // chunk_size
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
                let chunk_size = match quality {
                    ResamplingQuality::Balanced => 1024,
                    ResamplingQuality::High => 2048,
                    ResamplingQuality::Maximum => 4096,
                    _ => 1024,
                };

                if ratio >= 1.0 {
                    RubatoResamplerType::SincIn(
                        SincFixedIn::<f32>::new(
                            ratio,
                            2.0, // max_resample_ratio_relative
                            params,
                            chunk_size,
                            channels,
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
                            ratio,
                            2.0, // max_resample_ratio_relative
                            params,
                            chunk_size,
                            channels,
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

    /// Get expected input frame count
    fn input_frames_next(&self) -> usize {
        match &self.resampler {
            RubatoResamplerType::FastIn(r) => r.input_frames_next(),
            RubatoResamplerType::FastOut(r) => r.input_frames_next(),
            RubatoResamplerType::SincIn(r) => r.input_frames_next(),
            RubatoResamplerType::SincOut(r) => r.input_frames_next(),
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

        let input_frames = input.len() / self.channels;
        let mut output = Vec::new();
        let mut processed_frames = 0;

        // Process in chunks matching what the resampler expects
        while processed_frames < input_frames {
            let needed_frames = self.input_frames_next();
            let available_frames = input_frames - processed_frames;
            let frames_to_process = available_frames.min(needed_frames);

            if frames_to_process == 0 {
                break;
            }

            // Extract chunk
            let start_sample = processed_frames * self.channels;
            let end_sample = (processed_frames + frames_to_process) * self.channels;
            let chunk = &input[start_sample..end_sample];

            // Deinterleave chunk
            let input_channels = self.deinterleave(chunk, frames_to_process);

            // Process chunk
            let output_channels = match &mut self.resampler {
                RubatoResamplerType::FastIn(r) => {
                    if frames_to_process == needed_frames {
                        r.process(&input_channels, None).map_err(|e| {
                            ResamplingError::ProcessingFailed(format!(
                                "FastIn resampling failed: {}",
                                e
                            ))
                        })?
                    } else {
                        // Partial processing for last chunk
                        r.process_partial(Some(&input_channels), None)
                            .map_err(|e| {
                                ResamplingError::ProcessingFailed(format!(
                                    "FastIn partial resampling failed: {}",
                                    e
                                ))
                            })?
                    }
                }
                RubatoResamplerType::FastOut(r) => {
                    if frames_to_process == needed_frames {
                        r.process(&input_channels, None).map_err(|e| {
                            ResamplingError::ProcessingFailed(format!(
                                "FastOut resampling failed: {}",
                                e
                            ))
                        })?
                    } else {
                        r.process_partial(Some(&input_channels), None)
                            .map_err(|e| {
                                ResamplingError::ProcessingFailed(format!(
                                    "FastOut partial resampling failed: {}",
                                    e
                                ))
                            })?
                    }
                }
                RubatoResamplerType::SincIn(r) => {
                    if frames_to_process == needed_frames {
                        r.process(&input_channels, None).map_err(|e| {
                            ResamplingError::ProcessingFailed(format!(
                                "SincIn resampling failed: {}",
                                e
                            ))
                        })?
                    } else {
                        r.process_partial(Some(&input_channels), None)
                            .map_err(|e| {
                                ResamplingError::ProcessingFailed(format!(
                                    "SincIn partial resampling failed: {}",
                                    e
                                ))
                            })?
                    }
                }
                RubatoResamplerType::SincOut(r) => {
                    if frames_to_process == needed_frames {
                        r.process(&input_channels, None).map_err(|e| {
                            ResamplingError::ProcessingFailed(format!(
                                "SincOut resampling failed: {}",
                                e
                            ))
                        })?
                    } else {
                        r.process_partial(Some(&input_channels), None)
                            .map_err(|e| {
                                ResamplingError::ProcessingFailed(format!(
                                    "SincOut partial resampling failed: {}",
                                    e
                                ))
                            })?
                    }
                }
            };

            // Interleave and append to output
            output.extend(self.interleave(output_channels));

            processed_frames += frames_to_process;
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
        match &mut self.resampler {
            RubatoResamplerType::FastIn(r) => r.reset(),
            RubatoResamplerType::FastOut(r) => r.reset(),
            RubatoResamplerType::SincIn(r) => r.reset(),
            RubatoResamplerType::SincOut(r) => r.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rubato_creation() {
        let resampler =
            RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Balanced).unwrap();
        assert_eq!(resampler.input_rate(), 44100);
        assert_eq!(resampler.output_rate(), 96000);
        assert_eq!(resampler.channels(), 2);
    }

    #[test]
    fn test_deinterleave_interleave() {
        let resampler =
            RubatoResampler::new(44100, 48000, 2, ResamplingQuality::Fast).unwrap();

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
        let mut resampler =
            RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        let output = resampler.process(&[]).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_process_invalid_size() {
        let mut resampler =
            RubatoResampler::new(44100, 96000, 2, ResamplingQuality::Fast).unwrap();
        // Odd number of samples for stereo (should fail)
        let result = resampler.process(&[1.0, 2.0, 3.0]);
        assert!(result.is_err());
    }
}
