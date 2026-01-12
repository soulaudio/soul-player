//! Artwork writing functionality using lofty

use crate::error::{ArtworkError, Result};
use crate::types::ArtworkData;
use lofty::{MimeType, Picture, PictureType, TagExt, TaggedFileExt};
use std::path::Path;

/// Writes artwork to audio files
pub struct ArtworkWriter;

impl ArtworkWriter {
    /// Write artwork to an audio file's embedded metadata
    ///
    /// # Arguments
    /// * `file_path` - Path to the audio file
    /// * `artwork` - Artwork data to embed
    pub fn write_to_file(file_path: &Path, artwork: &ArtworkData) -> Result<()> {
        // Check if file exists
        if !file_path.exists() {
            return Err(ArtworkError::FileNotFound(file_path.to_path_buf()));
        }

        // Read the file with lofty (mutable for tag writing)
        let mut tagged_file = lofty::read_from_path(file_path)?;

        // Get or create primary tag - check primary first, then fall back to first tag
        let tag = if tagged_file.primary_tag_mut().is_some() {
            tagged_file.primary_tag_mut().unwrap()
        } else {
            tagged_file.first_tag_mut().ok_or(ArtworkError::NoTag)?
        };

        // Remove existing front cover pictures
        tag.remove_picture_type(PictureType::CoverFront);

        // Parse MIME type
        let mime_type = match artwork.mime_type.as_str() {
            "image/png" => MimeType::Png,
            "image/gif" => MimeType::Gif,
            "image/bmp" => MimeType::Bmp,
            "image/tiff" => MimeType::Tiff,
            _ => MimeType::Jpeg, // Default to JPEG
        };

        // Create new picture
        let picture = Picture::new_unchecked(
            PictureType::CoverFront,
            Some(mime_type),
            None, // description
            artwork.data.clone(),
        );

        // Add picture
        tag.push_picture(picture);

        // Save the tag back to the file
        tag.save_to_path(file_path)
            .map_err(|e: lofty::error::LoftyError| ArtworkError::SaveFailed(e.to_string()))?;

        Ok(())
    }

    /// Write artwork to multiple audio files
    ///
    /// Returns a vector of results, one for each file.
    ///
    /// # Arguments
    /// * `file_paths` - Paths to audio files
    /// * `artwork` - Artwork data to embed
    pub fn write_to_files<P: AsRef<Path>>(file_paths: &[P], artwork: &ArtworkData) -> Vec<Result<()>> {
        file_paths
            .iter()
            .map(|path| Self::write_to_file(path.as_ref(), artwork))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_nonexistent_file_returns_error() {
        let artwork = ArtworkData::new(vec![0, 1, 2], "image/jpeg".to_string());
        let result = ArtworkWriter::write_to_file(Path::new("/nonexistent/file.mp3"), &artwork);
        assert!(matches!(result, Err(ArtworkError::FileNotFound(_))));
    }
}
