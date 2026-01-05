//! Audio source implementations for desktop

pub mod local;
pub mod streaming;

pub use local::LocalAudioSource;
pub use streaming::StreamingAudioSource;
