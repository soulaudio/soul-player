//! Content hash computation for file relocation detection
//!
//! This module provides functionality for computing content hashes of audio files,
//! which enables detection of relocated files during library scanning. When a file
//! is moved to a different location, its hash remains the same, allowing the system
//! to update the file path rather than treating it as a new file.

use crate::{metadata, ImportError, Result};
use std::path::Path;

/// Compute content hash for a file with timeout protection
///
/// # Arguments
///
/// * `file_path` - Path to the file to hash
///
/// # Returns
///
/// * `Ok(Some(hash))` - Hash computed successfully
/// * `Ok(None)` - Hash computation disabled or skipped
/// * `Err` - Hash computation failed or timed out
///
/// # Errors
///
/// Returns an error if:
/// - Hash calculation times out (60 seconds)
/// - Hash calculation task panics
/// - Underlying I/O errors occur
pub async fn compute_file_hash(file_path: &Path) -> Result<String> {
    tracing::debug!("[HASH] Computing hash: {}", file_path.display());

    let file_path_owned = file_path.to_path_buf();
    let file_path_for_log = file_path.to_path_buf();

    let hash_task =
        tokio::task::spawn_blocking(move || metadata::calculate_file_hash(&file_path_owned));

    let hash = tokio::time::timeout(std::time::Duration::from_secs(60), hash_task)
        .await
        .map_err(|_| {
            tracing::error!(
                "[HASH] TIMEOUT computing hash: {}",
                file_path_for_log.display()
            );
            ImportError::Metadata(format!(
                "Hash calculation timeout (60s) for: {}",
                file_path_for_log.display()
            ))
        })?
        .map_err(|e| ImportError::Metadata(format!("Hash calculation task failed: {}", e)))??;

    Ok(hash)
}

/// Compute quick hash (first 64KB) with timeout protection.
/// Much faster than full content hash — suitable for initial dedup.
pub async fn compute_quick_hash(file_path: &Path) -> Result<String> {
    tracing::debug!("[HASH] Computing quick hash: {}", file_path.display());

    let file_path_owned = file_path.to_path_buf();
    let file_path_for_log = file_path.to_path_buf();

    let hash_task =
        tokio::task::spawn_blocking(move || metadata::calculate_quick_hash(&file_path_owned));

    let hash = tokio::time::timeout(std::time::Duration::from_secs(10), hash_task)
        .await
        .map_err(|_| {
            tracing::error!(
                "[HASH] TIMEOUT computing quick hash: {}",
                file_path_for_log.display()
            );
            ImportError::Metadata(format!(
                "Quick hash timeout (10s) for: {}",
                file_path_for_log.display()
            ))
        })?
        .map_err(|e| ImportError::Metadata(format!("Quick hash task failed: {}", e)))??;

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_compute_file_hash_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        let result = compute_file_hash(temp_file.path()).await;
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn test_compute_file_hash_consistency() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        let hash1 = compute_file_hash(temp_file.path()).await.unwrap();
        let hash2 = compute_file_hash(temp_file.path()).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_compute_file_hash_nonexistent_file() {
        let result = compute_file_hash(Path::new("/nonexistent/file.mp3")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_compute_quick_hash_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"quick hash test content").unwrap();
        temp_file.flush().unwrap();

        let result = compute_quick_hash(temp_file.path()).await;
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert!(!hash.is_empty());
        // SHA-256 hex string is 64 chars
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_compute_quick_hash_consistency() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"consistent content").unwrap();
        temp_file.flush().unwrap();

        let hash1 = compute_quick_hash(temp_file.path()).await.unwrap();
        let hash2 = compute_quick_hash(temp_file.path()).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_compute_quick_hash_differs_for_different_files() {
        let mut file1 = NamedTempFile::new().unwrap();
        file1.write_all(b"content of file one").unwrap();
        file1.flush().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"content of file two").unwrap();
        file2.flush().unwrap();

        let hash1 = compute_quick_hash(file1.path()).await.unwrap();
        let hash2 = compute_quick_hash(file2.path()).await.unwrap();

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_compute_quick_hash_nonexistent_file() {
        let result = compute_quick_hash(Path::new("/nonexistent/file.mp3")).await;
        assert!(result.is_err());
    }
}
