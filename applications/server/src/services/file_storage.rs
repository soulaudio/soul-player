/// File storage service - manages audio files on disk
use crate::{
    config::{AudioFormat, Quality},
    error::{Result, ServerError},
};
use soul_core::TrackId;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Initialize storage directories
    pub async fn initialize(&self) -> Result<()> {
        // Create quality subdirectories
        for quality in &[Quality::Original, Quality::High, Quality::Medium, Quality::Low] {
            let dir = self.base_path.join(quality.subdirectory());
            fs::create_dir_all(&dir).await?;
        }
        Ok(())
    }

    /// Store an original uploaded file
    pub async fn store_original(
        &self,
        track_id: &TrackId,
        extension: &str,
        data: &[u8],
    ) -> Result<PathBuf> {
        let filename = format!("{}.{}", track_id.as_str(), extension);
        let path = self.base_path.join("original").join(&filename);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&path, data).await?;
        Ok(path)
    }

    /// Store a transcoded variant
    pub async fn store_variant(
        &self,
        track_id: &TrackId,
        quality: Quality,
        format: AudioFormat,
        data: &[u8],
    ) -> Result<PathBuf> {
        let filename = format!("{}.{}", track_id.as_str(), format.extension());
        let path = self
            .base_path
            .join(quality.subdirectory())
            .join(&filename);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&path, data).await?;
        Ok(path)
    }

    /// Get the path to a track file
    pub fn get_track_path(
        &self,
        track_id: &TrackId,
        quality: Quality,
        format: Option<AudioFormat>,
    ) -> Result<PathBuf> {
        let quality_dir = self.base_path.join(quality.subdirectory());

        // If format is specified, look for that exact file
        if let Some(fmt) = format {
            let path = quality_dir.join(format!("{}.{}", track_id.as_str(), fmt.extension()));
            if path.exists() {
                return Ok(path);
            }
            return Err(ServerError::NotFound(format!(
                "Track file not found: {:?}",
                path
            )));
        }

        // Otherwise, find any supported format
        for fmt in &[
            AudioFormat::Mp3,
            AudioFormat::Flac,
            AudioFormat::Ogg,
            AudioFormat::Wav,
            AudioFormat::Opus,
        ] {
            let path = quality_dir.join(format!("{}.{}", track_id.as_str(), fmt.extension()));
            if path.exists() {
                return Ok(path);
            }
        }

        Err(ServerError::NotFound(format!(
            "No track file found for {} at quality {:?}",
            track_id.as_str(),
            quality
        )))
    }

    /// Check if a specific quality variant exists
    pub fn has_quality(&self, track_id: &TrackId, quality: Quality) -> bool {
        self.get_track_path(track_id, quality, None).is_ok()
    }

    /// Get the best available quality for a track
    pub fn get_best_available_quality(&self, track_id: &TrackId, requested: Quality) -> Quality {
        // Try requested quality first
        if self.has_quality(track_id, requested) {
            return requested;
        }

        // Fallback order based on requested quality
        let fallback_order = match requested {
            Quality::Original => vec![Quality::High, Quality::Medium, Quality::Low],
            Quality::High => vec![Quality::Original, Quality::Medium, Quality::Low],
            Quality::Medium => vec![Quality::High, Quality::Original, Quality::Low],
            Quality::Low => vec![Quality::Medium, Quality::High, Quality::Original],
        };

        for quality in fallback_order {
            if self.has_quality(track_id, quality) {
                return quality;
            }
        }

        // Default to original if nothing else exists
        Quality::Original
    }

    /// Delete all variants of a track
    pub async fn delete_track(&self, track_id: &TrackId) -> Result<()> {
        for quality in &[Quality::Original, Quality::High, Quality::Medium, Quality::Low] {
            let quality_dir = self.base_path.join(quality.subdirectory());

            // Try to delete files with all possible extensions
            for fmt in &[
                AudioFormat::Mp3,
                AudioFormat::Flac,
                AudioFormat::Ogg,
                AudioFormat::Wav,
                AudioFormat::Opus,
            ] {
                let path = quality_dir.join(format!("{}.{}", track_id.as_str(), fmt.extension()));
                if path.exists() {
                    fs::remove_file(&path).await?;
                }
            }
        }
        Ok(())
    }

    /// Validate that a path is within the storage directory (prevent directory traversal)
    pub fn validate_path(&self, path: &Path) -> Result<()> {
        let canonical_base = self
            .base_path
            .canonicalize()
            .map_err(|e| ServerError::Storage(format!("Invalid base path: {}", e)))?;

        let canonical_path = path
            .canonicalize()
            .map_err(|e| ServerError::Storage(format!("Invalid path: {}", e)))?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(ServerError::Unauthorized(
                "Path traversal attempt detected".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();

        let track_id = TrackId::generate();
        let data = b"fake audio data";

        // Store original
        storage
            .store_original(&track_id, "mp3", data)
            .await
            .unwrap();

        // Check it exists
        assert!(storage.has_quality(&track_id, Quality::Original));

        // Retrieve path
        let path = storage
            .get_track_path(&track_id, Quality::Original, Some(AudioFormat::Mp3))
            .unwrap();
        assert!(path.exists());
    }
}
