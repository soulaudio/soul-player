/// Server configuration
use crate::error::{Result, ServerError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_server")]
    pub server: ServerSettings,

    #[serde(default = "default_storage")]
    pub storage: StorageSettings,

    #[serde(default = "default_auth")]
    pub auth: AuthSettings,

    #[serde(default = "default_transcoding")]
    pub transcoding: TranscodingSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageSettings {
    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_music_storage_path")]
    pub music_storage_path: PathBuf,

    #[serde(default = "default_scan_directories")]
    pub scan_directories: Vec<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthSettings {
    pub jwt_secret: String,

    #[serde(default = "default_jwt_expiration_hours")]
    pub jwt_expiration_hours: u64,

    #[serde(default = "default_jwt_refresh_expiration_days")]
    pub jwt_refresh_expiration_days: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranscodingSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_formats")]
    pub formats: Vec<AudioFormat>,

    #[serde(default = "default_workers")]
    pub workers: usize,

    #[serde(default = "default_ffmpeg_path")]
    pub ffmpeg_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    Flac,
    Ogg,
    Wav,
    Opus,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Wav => "wav",
            AudioFormat::Opus => "opus",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Opus => "audio/opus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Original,
    High,
    Medium,
    Low,
}

impl Quality {
    pub fn subdirectory(&self) -> &'static str {
        match self {
            Quality::Original => "original",
            Quality::High => "high",
            Quality::Medium => "medium",
            Quality::Low => "low",
        }
    }
}

impl ServerConfig {
    /// Load configuration from file and environment
    pub fn load() -> Result<Self> {
        let mut settings = config::Config::builder();

        // Load from config file if it exists
        let config_path = PathBuf::from("config.toml");
        if config_path.exists() {
            settings = settings.add_source(config::File::from(config_path));
        }

        // Override with environment variables (prefixed with SOUL_)
        settings = settings.add_source(
            config::Environment::with_prefix("SOUL")
                .separator("_")
                .try_parsing(true),
        );

        let config = settings
            .build()
            .map_err(|e| ServerError::Config(e.to_string()))?;

        config
            .try_deserialize()
            .map_err(|e| ServerError::Config(e.to_string()))
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.auth.jwt_secret.is_empty() {
            return Err(ServerError::Config(
                "JWT secret is required (set SOUL_AUTH_JWT_SECRET)".to_string(),
            ));
        }

        if self.transcoding.enabled {
            if !self.transcoding.ffmpeg_path.exists() {
                return Err(ServerError::Config(format!(
                    "FFmpeg not found at {:?}",
                    self.transcoding.ffmpeg_path
                )));
            }
        }

        Ok(())
    }
}

// Default values
fn default_server() -> ServerSettings {
    ServerSettings {
        host: default_host(),
        port: default_port(),
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_storage() -> StorageSettings {
    StorageSettings {
        database_url: default_database_url(),
        music_storage_path: default_music_storage_path(),
        scan_directories: default_scan_directories(),
    }
}

fn default_database_url() -> String {
    "sqlite://./data/soul.db".to_string()
}

fn default_music_storage_path() -> PathBuf {
    PathBuf::from("./data/tracks")
}

fn default_scan_directories() -> Vec<PathBuf> {
    vec![]
}

fn default_auth() -> AuthSettings {
    AuthSettings {
        jwt_secret: String::new(),
        jwt_expiration_hours: default_jwt_expiration_hours(),
        jwt_refresh_expiration_days: default_jwt_refresh_expiration_days(),
    }
}

fn default_jwt_expiration_hours() -> u64 {
    24
}

fn default_jwt_refresh_expiration_days() -> u64 {
    30
}

fn default_transcoding() -> TranscodingSettings {
    TranscodingSettings {
        enabled: default_enabled(),
        formats: default_formats(),
        workers: default_workers(),
        ffmpeg_path: default_ffmpeg_path(),
    }
}

fn default_enabled() -> bool {
    true
}

fn default_formats() -> Vec<AudioFormat> {
    vec![AudioFormat::Mp3]
}

fn default_workers() -> usize {
    2
}

fn default_ffmpeg_path() -> PathBuf {
    PathBuf::from("/usr/bin/ffmpeg")
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            storage: default_storage(),
            auth: default_auth(),
            transcoding: default_transcoding(),
        }
    }
}
