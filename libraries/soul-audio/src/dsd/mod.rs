//! DSD (Direct Stream Digital) conversion
//!
//! Provides conversion between PCM and DSD formats:
//! - PCM to DSD conversion using sigma-delta modulation
//! - DSD to PCM conversion (future: for playback on non-DSD DACs)
//! - DoP (DSD over PCM) encoding/decoding
//!
//! # DSD Rates
//!
//! | Format  | Rate (MHz) | Multiplier | CD Equivalent |
//! |---------|------------|------------|---------------|
//! | DSD64   | 2.8224     | 64x        | 44100 Hz      |
//! | DSD128  | 5.6448     | 128x       | 44100 Hz      |
//! | DSD256  | 11.2896    | 256x       | 44100 Hz      |
//!
//! # Example: PCM to DSD Conversion
//!
//! ```rust
//! use soul_audio::dsd::{DsdConverter, DsdFormat};
//!
//! // Create converter for DSD128
//! let mut converter = DsdConverter::new(DsdFormat::Dsd128, 44100);
//!
//! // Convert stereo PCM samples
//! let pcm_samples: Vec<f32> = vec![0.5, 0.5, -0.3, -0.3]; // L, R, L, R...
//! let dsd_output = converter.process_pcm(&pcm_samples);
//!
//! // dsd_output contains packed DSD bits
//! println!("Generated {} DSD bytes", dsd_output.len());
//! ```

mod converter;
mod dop;
mod noise_shaper;

pub use converter::{DsdConverter, DsdFormat, DsdSettings};
pub use dop::{DoP, DopDecoder, DopEncoder};
pub use noise_shaper::{NoiseShaper, NoiseShaperOrder};
