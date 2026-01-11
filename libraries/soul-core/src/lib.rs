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

pub mod error;
pub mod storage;
pub mod traits;
pub mod types;

// Re-export commonly used types
pub use error::{Result, SoulError};
pub use storage::StorageContext;
pub use traits::{AudioDecoder, AudioEffect, AudioMetadata, AudioOutput, MetadataReader, Storage};

// Export all types
pub use types::{
    Album,
    AlbumId,
    Artist,
    ArtistId,
    // Audio types
    AudioBuffer,
    AudioFormat,
    AvailabilityStatus,
    CreateAlbum,
    CreateArtist,
    CreatePlaylist,
    CreateSource,
    CreateTrack,
    MetadataSource,
    Playlist,
    PlaylistId,
    PlaylistTrack,
    SampleRate,
    // Multi-source types (i64-based IDs)
    Source,
    SourceConfig,
    SourceId,
    SourceType,
    Track,
    TrackAvailability,
    TrackId,
    UpdateTrack,
    // User
    User,
    UserId,
};
