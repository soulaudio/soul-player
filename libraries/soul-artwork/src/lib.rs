//! Soul Artwork - Audio file artwork extraction library
//!
//! This library provides functionality to extract embedded artwork (album covers)
//! from audio files using the Lofty library. It supports various audio formats
//! including MP3 (ID3v2 APIC frames), FLAC (METADATA_BLOCK_PICTURE), and others.
//!
//! # Features
//!
//! - Extract embedded artwork from audio files
//! - LRU caching for performance
//! - Base64 encoding for web transfer
//! - Size limits to prevent memory issues
//!
//! # Example
//!
//! ```no_run
//! use soul_artwork::ArtworkExtractor;
//! use std::path::Path;
//!
//! let extractor = ArtworkExtractor::new(100); // Cache 100 images
//! let path = Path::new("music/track.mp3");
//!
//! match extractor.extract(path) {
//!     Ok(Some(artwork)) => {
//!         println!("Found artwork: {} bytes, type: {}",
//!             artwork.data.len(), artwork.mime_type);
//!     }
//!     Ok(None) => println!("No artwork found"),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

mod error;
mod extractor;
mod types;

// Re-export public API
pub use error::{ArtworkError, Result};
pub use extractor::ArtworkExtractor;
pub use types::ArtworkData;
