//! Loudness analysis and normalization for Soul Player
//!
//! This crate provides:
//! - EBU R128 loudness measurement (integrated LUFS, loudness range, true peak)
//! - ReplayGain 2.0 calculation (track and album gain)
//! - Tag reading/writing for ReplayGain metadata
//! - True peak limiting for playback
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌───────────────┐
//! │ Audio File  │ ──► │  Analyzer    │ ──► │ LoudnessInfo  │
//! └─────────────┘     └──────────────┘     └───────────────┘
//!                            │
//!                            ▼
//!                     ┌──────────────┐
//!                     │ Tag Writer   │
//!                     └──────────────┘
//!
//! During Playback:
//! ┌─────────────┐     ┌──────────────┐     ┌───────────────┐
//! │ Audio Data  │ ──► │ Gain Apply   │ ──► │ Peak Limiter  │
//! └─────────────┘     └──────────────┘     └───────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use soul_loudness::{LoudnessAnalyzer, ReplayGainCalculator};
//!
//! // Analyze a track
//! let mut analyzer = LoudnessAnalyzer::new(44100, 2)?;
//! analyzer.add_frames(&audio_samples)?;
//! let info = analyzer.finalize()?;
//!
//! println!("Integrated loudness: {:.1} LUFS", info.integrated_lufs);
//! println!("True peak: {:.1} dBTP", info.true_peak_dbfs);
//!
//! // Calculate ReplayGain
//! let calc = ReplayGainCalculator::new();
//! let rg = calc.track_gain(&info);
//! println!("Track gain: {:.2} dB", rg.gain_db);
//! ```

#![deny(unsafe_code)]

mod analyzer;
mod error;
mod limiter;
mod normalizer;
mod replaygain;
mod tags;

pub use analyzer::{LoudnessAnalyzer, LoudnessInfo};
pub use error::{LoudnessError, Result};
pub use limiter::TruePeakLimiter;
pub use normalizer::{LoudnessNormalizer, NormalizationMode};
pub use replaygain::{AlbumGain, ReplayGainCalculator, TrackGain};
pub use tags::{read_replaygain_tags, write_replaygain_tags, ReplayGainTags};

/// ReplayGain 2.0 reference loudness level (-18 LUFS)
/// This is the target loudness for normalized audio
pub const REPLAYGAIN_REFERENCE_LUFS: f64 = -18.0;

/// EBU R128 broadcast reference level (-23 LUFS)
pub const EBU_R128_BROADCAST_LUFS: f64 = -23.0;

/// EBU R128 streaming reference level (-14 LUFS, common for streaming platforms)
pub const EBU_R128_STREAMING_LUFS: f64 = -14.0;

/// Maximum pre-amp gain in dB
pub const MAX_PREAMP_DB: f64 = 12.0;

/// Minimum pre-amp gain in dB
pub const MIN_PREAMP_DB: f64 = -12.0;
