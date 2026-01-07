//! Common types for the importer

use serde::{Deserialize, Serialize};
use soul_core::types::{Album, Artist, Genre};
use std::path::PathBuf;

/// File management strategy for imports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileManagementStrategy {
    /// Move files to managed library (recommended - saves disk space)
    Move,

    /// Copy files to managed library (recommended - preserves originals)
    #[default]
    Copy,

    /// Reference files in current location (warning: breaks if files move)
    Reference,
}

/// Configuration for import operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportConfig {
    /// Path to the managed library folder (e.g., ~/Music/soul-player/library)
    pub library_path: PathBuf,

    /// File management strategy
    pub file_strategy: FileManagementStrategy,

    /// Minimum confidence score to auto-accept fuzzy matches (0-100)
    /// Matches below this require user review
    pub confidence_threshold: u8,

    /// File naming pattern (e.g., "{artist} - {title}.{ext}")
    pub file_naming_pattern: String,

    /// Whether to skip duplicate files
    pub skip_duplicates: bool,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            library_path: PathBuf::from("~/Music/soul-player/library"),
            file_strategy: FileManagementStrategy::Copy,
            confidence_threshold: 85,
            file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
            skip_duplicates: true,
        }
    }
}

/// Result of a fuzzy match operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyMatch<T> {
    /// The matched or created entity
    pub entity: T,

    /// Confidence score (0-100)
    pub confidence: u8,

    /// Type of match that occurred
    pub match_type: MatchType,
}

/// Type of fuzzy match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// Exact match (100% confidence)
    Exact,

    /// Normalized match - case/whitespace/punctuation differences (90-95%)
    Normalized,

    /// Fuzzy match - Levenshtein distance within threshold (60-89%)
    Fuzzy,

    /// New entity created - no match found
    Created,
}

/// Result of importing a single track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// ID of the imported track
    pub track_id: i64,

    /// Original file path
    pub source_path: PathBuf,

    /// New file path in managed library
    pub library_path: PathBuf,

    /// Artist match result
    pub artist_match: Option<FuzzyMatch<Artist>>,

    /// Album match result
    pub album_match: Option<FuzzyMatch<Album>>,

    /// Genre match results
    pub genre_matches: Vec<FuzzyMatch<Genre>>,

    /// Whether this track requires user review (low confidence matches)
    pub requires_review: bool,

    /// Any warnings or non-fatal errors
    pub warnings: Vec<String>,
}

/// Progress update during import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    /// Total number of files to import
    pub total_files: usize,

    /// Number of files processed so far
    pub processed_files: usize,

    /// Number of files successfully imported
    pub successful_imports: usize,

    /// Number of files skipped (duplicates)
    pub skipped_duplicates: usize,

    /// Number of files that failed to import
    pub failed_imports: usize,

    /// Current file being processed
    pub current_file: Option<PathBuf>,

    /// Estimated time remaining in seconds
    pub estimated_seconds_remaining: Option<u64>,
}

impl ImportProgress {
    pub fn new(total_files: usize) -> Self {
        Self {
            total_files,
            processed_files: 0,
            successful_imports: 0,
            skipped_duplicates: 0,
            failed_imports: 0,
            current_file: None,
            estimated_seconds_remaining: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.processed_files >= self.total_files
    }

    pub fn percentage(&self) -> f32 {
        if self.total_files == 0 {
            return 100.0;
        }
        (self.processed_files as f32 / self.total_files as f32) * 100.0
    }
}

/// Summary of an import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    /// Total files processed
    pub total_processed: usize,

    /// Successfully imported
    pub successful: usize,

    /// Skipped as duplicates
    pub duplicates_skipped: usize,

    /// Failed to import
    pub failed: usize,

    /// Tracks requiring user review
    pub require_review: Vec<ImportResult>,

    /// Error messages for failed imports
    pub errors: Vec<(PathBuf, String)>,

    /// Duration of import operation
    pub duration_seconds: u64,
}

impl ImportSummary {
    pub fn summary_text(&self) -> String {
        format!(
            "Import complete: {} successful, {} duplicates skipped, {} failed, {} require review",
            self.successful,
            self.duplicates_skipped,
            self.failed,
            self.require_review.len()
        )
    }
}
