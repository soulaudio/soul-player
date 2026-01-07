//! Soul Player Audio
//!
//! Audio decoding, playback, and effects processing for Soul Player.
//!
//! This crate provides:
//! - Audio decoding via Symphonia (MP3, FLAC, OGG, WAV, AAC, OPUS)
//! - Real-time audio effects (3-band parametric EQ, dynamic range compressor)
//! - Effect chain architecture for combining multiple effects
//!
//! # Example: Decoding Audio
//!
//! ```rust,no_run
//! use soul_audio::SymphoniaDecoder;
//! use soul_core::AudioDecoder;
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Decode an audio file
//! let mut decoder = SymphoniaDecoder::new();
//! let buffer = decoder.decode(Path::new("/music/song.mp3"))?;
//!
//! println!("Decoded {} samples at {} Hz", buffer.len(), buffer.format.sample_rate.as_hz());
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Using Effects
//!
//! ```rust
//! use soul_audio::effects::{EffectChain, ParametricEq, Compressor, EqBand, CompressorSettings};
//!
//! // Create effect chain
//! let mut chain = EffectChain::new();
//!
//! // Add 3-band EQ
//! let mut eq = ParametricEq::new();
//! eq.set_low_band(EqBand::low_shelf(80.0, 3.0));  // Boost bass
//! eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));  // Cut mids
//! eq.set_high_band(EqBand::high_shelf(8000.0, 2.0));  // Boost treble
//! chain.add_effect(Box::new(eq));
//!
//! // Add compressor
//! let comp = Compressor::with_settings(CompressorSettings::moderate());
//! chain.add_effect(Box::new(comp));
//!
//! // Process audio
//! let mut buffer = vec![0.0; 1024]; // Stereo samples
//! chain.process(&mut buffer, 44100);
//! ```

mod decoder;
pub mod effects;
mod error;

pub use decoder::SymphoniaDecoder;
pub use error::{AudioError, Result};
