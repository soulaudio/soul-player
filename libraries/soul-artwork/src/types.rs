use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Artwork data extracted from an audio file
#[derive(Debug, Clone)]
pub struct ArtworkData {
    /// Raw image bytes
    pub data: Vec<u8>,
    /// MIME type (e.g., "image/jpeg", "image/png")
    pub mime_type: String,
}

impl ArtworkData {
    /// Create new artwork data
    pub fn new(data: Vec<u8>, mime_type: String) -> Self {
        Self { data, mime_type }
    }

    /// Get the data as a base64-encoded string
    pub fn to_base64(&self) -> String {
        STANDARD.encode(&self.data)
    }
}
