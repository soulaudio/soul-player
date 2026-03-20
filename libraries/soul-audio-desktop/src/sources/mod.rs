//! Audio source implementations for desktop

pub mod dsd;
pub mod local;
pub mod streaming;

pub use dsd::DsdAudioSource;
pub use local::LocalAudioSource;
pub use streaming::StreamingAudioSource;
