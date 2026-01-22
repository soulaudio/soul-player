use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration stored in config.json
/// This is used to cache settings that need to be read before the database is initialized
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    /// Enable file logging (cached from database preference)
    #[serde(default)]
    pub enable_file_logging: bool,
}

impl AppConfig {
    /// Read config from config.json in the app data directory
    /// Returns None if the file doesn't exist or can't be parsed
    pub fn read(app_data_dir: &std::path::Path) -> Option<Self> {
        let config_path = app_data_dir.join("config.json");

        tracing::debug!("Reading config from: {}", config_path.display());

        let content = std::fs::read_to_string(&config_path).ok()?;
        let config: AppConfig = serde_json::from_str(&content).ok()?;

        tracing::debug!(
            "Config loaded: enable_file_logging={}",
            config.enable_file_logging
        );

        Some(config)
    }

    /// Write config to config.json in the app data directory
    pub fn write(&self, app_data_dir: &std::path::Path) -> Result<(), std::io::Error> {
        let config_path = app_data_dir.join("config.json");

        tracing::debug!("Writing config to: {}", config_path.display());
        tracing::debug!("Config: enable_file_logging={}", self.enable_file_logging);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, json)?;

        tracing::debug!("Config written successfully");

        Ok(())
    }

    /// Get the config file path for a given app data directory
    pub fn get_path(app_data_dir: &std::path::Path) -> PathBuf {
        app_data_dir.join("config.json")
    }
}
