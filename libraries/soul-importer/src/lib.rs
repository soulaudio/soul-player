//! Soul Player Music Importer
//!
//! This crate handles importing music files into the Soul Player library.
//!
//! # Features
//!
//! - File scanning (folders, individual files, drag & drop)
//! - Metadata extraction from audio tags
//! - Fuzzy matching for artists, albums, and genres with confidence scoring
//! - File copying to managed library with organized naming
//! - Duplicate detection via file hashing
//! - Progress reporting
//! - Background import processing
//!
//! # Architecture
//!
//! - `scanner`: Filesystem scanning for audio files
//! - `metadata`: Metadata extraction from audio tags
//! - `fuzzy`: Fuzzy matching with confidence scoring
//! - `copy`: File copying to managed library
//! - `importer`: Orchestration of the import process
//! - `paid`: Stub interfaces for paid features (MusicBrainz, AcoustID)

mod error;
mod types;

// Core modules
pub mod copy;
pub mod fuzzy;
pub mod importer;
pub mod library_scanner;
pub mod metadata;
pub mod scanner;

// Paid features (stubbed)
pub mod paid;

pub use error::ImportError;
pub use importer::MusicImporter;
pub use types::*;

/// Re-export commonly used types
pub type Result<T> = std::result::Result<T, ImportError>;
