//! Soul Player - Playback Management
//!
//! Platform-agnostic playback management for Soul Player.
//!
//! This crate provides:
//! - Volume control (logarithmic, 0-100%, mute/unmute)
//! - Two-tier queue system (explicit + source)
//! - Playback history (configurable size)
//! - Shuffle algorithms (Random + Smart)
//! - Repeat modes (Off, All, One)
//! - Seek functionality (time and percentage)
//! - Audio effects integration
//! - Gapless playback support
//!
//! # Architecture
//!
//! `soul-playback` is completely platform-agnostic:
//! - No dependency on CPAL (desktop audio)
//! - No dependency on Tauri (desktop UI)
//! - No dependency on soul-storage (database)
//! - Works on desktop, ESP32, and server
//!
//! Platform-specific code (audio output, track loading) is provided via traits.
//!
//! # Example: Basic Playback
//!
//! ```rust
//! use soul_playback::{PlaybackManager, PlaybackConfig, QueueTrack};
//! use std::time::Duration;
//! use std::path::PathBuf;
//!
//! // Create playback manager
//! let mut manager = PlaybackManager::new(PlaybackConfig::default());
//!
//! // Set volume to 80%
//! manager.set_volume(80);
//!
//! // Add tracks to queue
//! # use soul_playback::types::TrackSource;
//! let track = QueueTrack {
//!     id: "track1".to_string(),
//!     path: PathBuf::from("/music/song.mp3"),
//!     title: "My Song".to_string(),
//!     artist: "Artist Name".to_string(),
//!     album: Some("Album Name".to_string()),
//!     duration: Duration::from_secs(180),
//!     track_number: Some(1),
//!     source: TrackSource::Single,
//! };
//!
//! manager.add_to_queue_end(track);
//!
//! // Control playback
//! // manager.play().ok();  // Platform loads audio source
//! // manager.pause();
//! // manager.next().ok();
//! // manager.previous().ok();
//! ```
//!
//! # Example: Shuffle and Repeat
//!
//! ```rust
//! use soul_playback::{PlaybackManager, types::{ShuffleMode, RepeatMode}};
//!
//! let mut manager = PlaybackManager::default();
//!
//! // Enable smart shuffle
//! manager.set_shuffle(ShuffleMode::Smart);
//!
//! // Enable repeat all
//! manager.set_repeat(RepeatMode::All);
//! ```
//!
//! # Example: Platform Integration
//!
//! ```rust,no_run
//! use soul_playback::{PlaybackManager, AudioSource, Result};
//! use std::time::Duration;
//!
//! // Implement AudioSource for your platform
//! struct MyAudioDecoder {
//!     // ... platform-specific decoder
//! }
//!
//! impl AudioSource for MyAudioDecoder {
//!     fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
//!         // Decode audio samples
//!         Ok(buffer.len())
//!     }
//!
//!     fn seek(&mut self, position: Duration) -> Result<()> {
//!         // Seek in audio file
//!         Ok(())
//!     }
//!
//!     fn duration(&self) -> Duration {
//!         Duration::from_secs(180)
//!     }
//!
//!     fn position(&self) -> Duration {
//!         Duration::from_secs(0)
//!     }
//!
//!     fn is_finished(&self) -> bool {
//!         false
//!     }
//! }
//!
//! // Use with playback manager
//! let mut manager = PlaybackManager::default();
//! let decoder = MyAudioDecoder { /* ... */ };
//! manager.set_audio_source(Box::new(decoder));
//!
//! // Process audio in platform audio callback
//! let mut output_buffer = vec![0.0f32; 1024];
//! manager.process_audio(&mut output_buffer).ok();
//! ```

mod error;
mod history;
mod manager;
mod queue;
mod shuffle;
mod source;
pub mod types;
mod volume;

// Public exports
pub use error::{PlaybackError, Result};
pub use manager::PlaybackManager;
pub use source::AudioSource;
pub use types::{PlaybackConfig, PlaybackState, QueueTrack, RepeatMode, ShuffleMode, TrackSource};
