//! Soul Player Core
//!
//! Platform-agnostic core types, traits, and error handling for Soul Player.
//!
//! This crate provides the foundational building blocks used across all platforms
//! (desktop, server, and embedded).
//!
//! # Architecture
//!
//! The core crate defines:
//! - **Domain Types**: `Track`, `Playlist`, `User`, etc.
//! - **Core Traits**: `Storage`, `AudioDecoder`, `AudioOutput`, `AudioEffect`
//! - **Error Handling**: Unified `SoulError` and `Result` types
//!
//! # Example
//!
//! ```rust
//! use soul_core::types::{Track, User, Playlist};
//! use soul_core::types::{UserId, TrackId, PlaylistId};
//! use std::path::PathBuf;
//!
//! // Create a user
//! let user = User::new("Alice");
//!
//! // Create a track
//! let track = Track::new("My Favorite Song", PathBuf::from("/music/song.mp3"));
//!
//! // Create a playlist
//! let playlist = Playlist::new(user.id.clone(), "My Favorites");
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod traits;
pub mod types;
pub mod storage;

// Re-export commonly used types
pub use error::{Result, SoulError};
pub use traits::{AudioDecoder, AudioEffect, AudioOutput, MetadataReader, Storage};
pub use storage::StorageContext;

// Export all types
pub use types::{
    // Audio types
    AudioBuffer, AudioFormat, SampleRate,
    // User
    User,
    // Multi-source types (i64-based IDs)
    Source, SourceType, SourceConfig, CreateSource, SourceId,
    Artist, CreateArtist, ArtistId,
    Album, CreateAlbum, AlbumId,
    Track, TrackId, CreateTrack, UpdateTrack, TrackAvailability, AvailabilityStatus, MetadataSource,
    Playlist, CreatePlaylist, PlaylistTrack,
    PlaylistId, UserId,
};
