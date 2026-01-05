/// Metadata reader implementation using lofty
use crate::error::MetadataError;
use lofty::{AudioFile, TaggedFileExt};
use soul_core::{MetadataReader, TrackMetadata};
use std::path::Path;

/// Metadata reader using the lofty library
pub struct LoftyMetadataReader;

impl LoftyMetadataReader {
    /// Create a new metadata reader
    pub fn new() -> Self {
        Self
    }

    /// Extract metadata from lofty tag
    fn extract_from_tag(tag: &lofty::Tag) -> TrackMetadata {
        let mut metadata = TrackMetadata::new();

        // lofty 0.18 API - iterate through items
        for item in tag.items() {
            match item.key() {
                lofty::ItemKey::TrackTitle => {
                    metadata.title = item.value().text().map(|s| s.to_string());
                }
                lofty::ItemKey::TrackArtist => {
                    metadata.artist = item.value().text().map(|s| s.to_string());
                }
                lofty::ItemKey::AlbumTitle => {
                    metadata.album = item.value().text().map(|s| s.to_string());
                }
                lofty::ItemKey::AlbumArtist => {
                    metadata.album_artist = item.value().text().map(|s| s.to_string());
                }
                lofty::ItemKey::Genre => {
                    metadata.genre = item.value().text().map(|s| s.to_string());
                }
                lofty::ItemKey::Year => {
                    if let Some(text) = item.value().text() {
                        metadata.year = text.parse().ok();
                    }
                }
                lofty::ItemKey::TrackNumber => {
                    if let Some(text) = item.value().text() {
                        metadata.track_number = text.parse().ok();
                    }
                }
                lofty::ItemKey::DiscNumber => {
                    if let Some(text) = item.value().text() {
                        metadata.disc_number = text.parse().ok();
                    }
                }
                _ => {}
            }
        }

        metadata
    }
}

impl Default for LoftyMetadataReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataReader for LoftyMetadataReader {
    fn read(&self, path: &Path) -> soul_core::Result<TrackMetadata> {
        // Check if file exists
        if !path.exists() {
            return Err(MetadataError::FileNotFound(path.display().to_string()).into());
        }

        // Probe and read the file
        let tagged_file = lofty::read_from_path(path)
            .map_err(|e| soul_core::SoulError::metadata(e.to_string()))?;

        // Extract duration from properties
        let duration_ms = Some(tagged_file.properties().duration().as_millis() as u64);

        // Get primary tag or default tag (if available)
        let metadata = if let Some(primary) = tagged_file.primary_tag() {
            let mut meta = Self::extract_from_tag(primary);
            meta.duration_ms = duration_ms;
            meta
        } else if let Some(first) = tagged_file.tags().first() {
            let mut meta = Self::extract_from_tag(first);
            meta.duration_ms = duration_ms;
            meta
        } else {
            // No tags found - return empty metadata with just duration
            let mut meta = TrackMetadata::new();
            meta.duration_ms = duration_ms;
            meta
        };

        Ok(metadata)
    }

    fn write(&self, _path: &Path, _metadata: &TrackMetadata) -> soul_core::Result<()> {
        // For MVP, writing is not implemented
        // Will be added when tag editing feature is needed
        Err(MetadataError::WriteError("Tag writing not yet implemented".to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_creation() {
        let reader = LoftyMetadataReader::new();
        // Just verify it compiles and constructs
        drop(reader);
    }

    #[test]
    fn read_nonexistent_file_returns_error() {
        let reader = LoftyMetadataReader::new();
        let result = reader.read(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }
}
