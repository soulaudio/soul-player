//! Paid features stub - online metadata lookup and acoustic fingerprinting
//!
//! These features require subscriptions or API keys and will be implemented
//! in future phases.

use crate::Result;
use serde::{Deserialize, Serialize};

/// Configuration for paid features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaidFeaturesConfig {
    /// MusicBrainz API key (if using paid tier)
    pub musicbrainz_api_key: Option<String>,

    /// AcoustID API key for fingerprinting
    pub acoustid_api_key: Option<String>,

    /// Discogs API key
    pub discogs_api_key: Option<String>,

    /// Whether paid features are enabled
    pub enabled: bool,
}

impl Default for PaidFeaturesConfig {
    fn default() -> Self {
        Self {
            musicbrainz_api_key: None,
            acoustid_api_key: None,
            discogs_api_key: None,
            enabled: false,
        }
    }
}

/// Metadata from online databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub album_art_url: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub confidence: u8,
}

/// Stub: Query MusicBrainz/Discogs for metadata by artist and title
///
/// TODO: Implement actual MusicBrainz/Discogs API calls
pub async fn lookup_metadata(
    _config: &PaidFeaturesConfig,
    _artist: &str,
    _title: &str,
) -> Result<Option<OnlineMetadata>> {
    // Stubbed out - will be implemented when soul-discovery is integrated
    tracing::warn!("Online metadata lookup is not yet implemented");
    Ok(None)
}

/// Stub: Generate acoustic fingerprint and lookup via AcoustID
///
/// TODO: Implement Chromaprint fingerprinting + AcoustID lookup
pub async fn fingerprint_lookup(
    _config: &PaidFeaturesConfig,
    _file_path: &std::path::Path,
) -> Result<Option<OnlineMetadata>> {
    // Stubbed out - requires Chromaprint integration
    tracing::warn!("Acoustic fingerprinting is not yet implemented");
    Ok(None)
}

/// Stub: Download album art from URL
///
/// TODO: Implement album art downloading
pub async fn download_album_art(_url: &str, _destination: &std::path::Path) -> Result<()> {
    // Stubbed out
    tracing::warn!("Album art download is not yet implemented");
    Ok(())
}

/// Stub: Batch update metadata for multiple tracks
///
/// TODO: Implement batch metadata update UI and backend
pub async fn batch_update_metadata(
    _config: &PaidFeaturesConfig,
    _track_ids: &[i64],
) -> Result<Vec<OnlineMetadata>> {
    // Stubbed out
    tracing::warn!("Batch metadata update is not yet implemented");
    Ok(Vec::new())
}
