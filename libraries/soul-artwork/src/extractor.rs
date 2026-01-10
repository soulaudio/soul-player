use crate::error::{ArtworkError, Result};
use crate::types::ArtworkData;
use lofty::{PictureType, TaggedFileExt};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Maximum artwork size (5MB)
const MAX_ARTWORK_SIZE: usize = 5 * 1024 * 1024;

/// Extracts artwork from audio files with LRU caching
pub struct ArtworkExtractor {
    cache: Arc<Mutex<LruCache<PathBuf, Arc<ArtworkData>>>>,
}

impl ArtworkExtractor {
    /// Create a new artwork extractor with the specified cache size
    ///
    /// # Arguments
    /// * `cache_size` - Maximum number of images to cache (0 to disable caching)
    pub fn new(cache_size: usize) -> Self {
        let cache = if cache_size > 0 {
            LruCache::new(NonZeroUsize::new(cache_size).unwrap())
        } else {
            LruCache::new(NonZeroUsize::new(1).unwrap())
        };

        Self {
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Extract artwork from an audio file
    ///
    /// Returns `Ok(Some(artwork))` if artwork found, `Ok(None)` if no artwork,
    /// or `Err` if there was an error reading the file.
    ///
    /// # Arguments
    /// * `path` - Path to the audio file
    pub fn extract(&self, path: &Path) -> Result<Option<ArtworkData>> {
        // Canonicalize path for consistent cache keys
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&canonical_path) {
                return Ok(Some((**cached).clone()));
            }
        }

        // Not in cache, extract from file
        match Self::extract_from_file(path) {
            Ok(Some(artwork)) => {
                // Store in cache
                let arc_artwork = Arc::new(artwork.clone());
                let mut cache = self.cache.lock().unwrap();
                cache.put(canonical_path, arc_artwork);
                Ok(Some(artwork))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Extract artwork and encode as base64
    ///
    /// Convenience method that combines `extract()` and base64 encoding.
    ///
    /// # Arguments
    /// * `path` - Path to the audio file
    pub fn extract_base64(&self, path: &Path) -> Result<Option<String>> {
        match self.extract(path)? {
            Some(artwork) => Ok(Some(artwork.to_base64())),
            None => Ok(None),
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Extract artwork from a file without caching
    fn extract_from_file(path: &Path) -> Result<Option<ArtworkData>> {
        // Check if file exists
        if !path.exists() {
            return Err(ArtworkError::FileNotFound(path.to_path_buf()));
        }

        // Read the file with lofty
        let tagged_file = lofty::read_from_path(path)?;

        // Get primary tag or first available tag
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let Some(tag) = tag else {
            return Ok(None);
        };

        // Extract pictures from tag
        let pictures = tag.pictures();
        if pictures.is_empty() {
            return Ok(None);
        }

        // Prefer front cover, otherwise use first picture
        let picture = pictures
            .iter()
            .find(|p| matches!(p.pic_type(), PictureType::CoverFront))
            .or_else(|| pictures.first());

        let Some(picture) = picture else {
            return Ok(None);
        };

        // Check size limit
        let data = picture.data();
        if data.len() > MAX_ARTWORK_SIZE {
            eprintln!(
                "Warning: Artwork in {} is too large ({} bytes, max {} bytes), skipping",
                path.display(),
                data.len(),
                MAX_ARTWORK_SIZE
            );
            return Err(ArtworkError::TooLarge(data.len(), MAX_ARTWORK_SIZE));
        }

        // Get MIME type (default to "image/jpeg" if not specified)
        let mime_type = picture
            .mime_type()
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "image/jpeg".to_string());

        Ok(Some(ArtworkData::new(data.to_vec(), mime_type)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractor_creation() {
        let extractor = ArtworkExtractor::new(10);
        assert!(extractor.cache.lock().unwrap().is_empty());
    }

    #[test]
    fn extract_nonexistent_file_returns_error() {
        let extractor = ArtworkExtractor::new(10);
        let result = extractor.extract(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }

    #[test]
    fn clear_cache_works() {
        let extractor = ArtworkExtractor::new(10);
        extractor.clear_cache();
        assert!(extractor.cache.lock().unwrap().is_empty());
    }
}
