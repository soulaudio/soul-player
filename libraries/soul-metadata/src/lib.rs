//! Soul Player Metadata
//!
//! Metadata extraction and library scanning for Soul Player.
//!
//! This crate provides:
//! - Tag reading from audio files (MP3, FLAC, OGG, WAV, AAC, OPUS)
//! - Library scanning with progress reporting
//! - Incremental scanning support
//! - Multi-threaded processing (configurable)
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_metadata::{LoftyMetadataReader, LibraryScanner};
//! use soul_core::MetadataReader;
//! use std::path::Path;
//! use std::sync::Arc;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Read metadata from a file
//! let reader = LoftyMetadataReader::new();
//! let metadata = reader.read(Path::new("/music/song.mp3"))?;
//!
//! // Scan a library (requires database)
//! // let scanner = LibraryScanner::new(db);
//! // let stats = scanner.scan(Path::new("/music"), None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Future: soul-import
//!
//! A separate `soul-import` crate will handle:
//! - Metadata normalization and fixing
//! - Acoustic fingerprinting
//! - MusicBrainz integration
//! - Batch tag editing
//! - Import from streaming services

mod error;
mod reader;
mod scanner;

pub use error::{MetadataError, Result};
pub use reader::LoftyMetadataReader;
pub use scanner::{LibraryScanner, ScanConfig, ScanProgress, ScanStats};
