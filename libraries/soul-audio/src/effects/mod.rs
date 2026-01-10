///! Audio effects processing
///!
///! This module provides a trait-based effect chain architecture for real-time audio processing.
///! All effects operate on f32 samples in [-1.0, 1.0] range.
///!
///! Available effects:
///! - **ParametricEq**: 3-band parametric equalizer
///! - **GraphicEq**: 10-band or 31-band graphic equalizer
///! - **Compressor**: Dynamic range compressor
///! - **Limiter**: Brick-wall limiter
///! - **Crossfeed**: Bauer stereophonic-to-binaural DSP for headphones
///! - **StereoEnhancer**: Width control, mid/side processing, balance

mod chain;
mod compressor;
mod crossfeed;
mod eq;
mod graphic_eq;
mod limiter;
mod stereo;

pub use chain::{AudioEffect, EffectChain};
pub use compressor::{Compressor, CompressorSettings};
pub use crossfeed::{Crossfeed, CrossfeedPreset, CrossfeedSettings};
pub use eq::{EqBand, ParametricEq};
pub use graphic_eq::{GraphicEq, GraphicEqBands, GraphicEqPreset, ISO_10_BAND_FREQUENCIES, ISO_31_BAND_FREQUENCIES};
pub use limiter::{Limiter, LimiterSettings};
pub use stereo::{StereoEnhancer, StereoSettings, mono_compatibility};

#[cfg(test)]
mod tests {
    /// Generate a sine wave for testing
    pub(crate) fn generate_sine(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(num_samples * 2); // Stereo

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * freq * t).sin();
            samples.push(sample); // Left
            samples.push(sample); // Right
        }

        samples
    }
}
