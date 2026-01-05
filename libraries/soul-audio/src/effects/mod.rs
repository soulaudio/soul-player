///! Audio effects processing
///!
///! This module provides a trait-based effect chain architecture for real-time audio processing.
///! All effects operate on f32 samples in [-1.0, 1.0] range.

mod eq;
mod compressor;
mod chain;

pub use eq::{ParametricEq, EqBand};
pub use compressor::{Compressor, CompressorSettings};
pub use chain::{EffectChain, AudioEffect};

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
