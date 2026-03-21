//! DSD audio source — supports DSF and DSDIFF file formats.
//!
//! # Architecture
//!
//! ```text
//! DsdAudioSource (implements AudioSource)
//!   ├── background thread: reads DSD bytes → Dsd2Pcm FIR filter → rtrb ring buffer
//!   └── audio callback thread: non-blocking reads from ring buffer
//!
//! Container layer:
//!   DsfContainer  (dsf-meta crate)  ← DSF files (.dsf)
//!   DsdiffContainer (dff-meta crate) ← DSDIFF files (.dff, .dsdiff)
//! ```
//!
//! # Output sample rate
//!
//! The `Dsd2Pcm` FIR filter produces one PCM sample per 8 DSD bits (one byte).
//! Output rate = `dsd_rate / 8`:
//! - DSD64  (2,822,400 Hz)  → 352,800 Hz PCM
//! - DSD128 (5,644,800 Hz)  → 705,600 Hz PCM
//!
//! The existing resampler downstream converts this to the device's target rate.

pub mod container;
pub mod source;

pub use source::DsdAudioSource;
