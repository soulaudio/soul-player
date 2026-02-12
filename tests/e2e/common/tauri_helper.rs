//! Tauri WebDriver integration helpers for E2E audio tests
//!
//! This module provides utilities for:
//! - Launching Soul Player with WebDriver
//! - Interacting with UI elements
//! - Loading test tracks
//! - Querying playback state

use anyhow::{Context, Result};
use fantoccini::{Client, ClientBuilder, Locator};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Tauri WebDriver session
///
/// Manages the lifecycle of a Tauri app instance for testing.
/// Automatically cleans up on drop.
pub struct TauriDriver {
    /// WebDriver client
    client: Client,
    /// tauri-driver process
    _driver_process: Option<Child>,
    /// Temporary directory for app data (isolated from user's data)
    _temp_dir: TempDir,
}

impl TauriDriver {
    /// Launch Soul Player with WebDriver
    ///
    /// This starts:
    /// 1. tauri-driver server
    /// 2. Soul Player app with WebDriver enabled
    /// 3. WebDriver client connection
    pub async fn new() -> Result<Self> {
        // Create isolated temp directory for app data
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let app_data_dir = temp_dir.path().join("soul-player-test");
        std::fs::create_dir_all(&app_data_dir).context("Failed to create app data directory")?;

        // Start tauri-driver
        tracing::info!("[TauriDriver] Starting tauri-driver...");
        let driver_process = Command::new("tauri-driver")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(
                "Failed to start tauri-driver. Is it installed? Run: cargo install tauri-driver",
            )?;

        // Wait for driver to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Build WebDriver client with Tauri app capabilities
        tracing::info!("[TauriDriver] Connecting WebDriver client...");
        let capabilities = Self::build_capabilities(&app_data_dir)?;

        let client = ClientBuilder::native()
            .capabilities(capabilities)
            .connect("http://localhost:4444")
            .await
            .context("Failed to connect to tauri-driver. Make sure it's running.")?;

        tracing::info!("[TauriDriver] WebDriver session established");

        Ok(Self {
            client,
            _driver_process: Some(driver_process),
            _temp_dir: temp_dir,
        })
    }

    /// Build WebDriver capabilities for Tauri app
    fn build_capabilities(app_data_dir: &Path) -> Result<serde_json::Map<String, Value>> {
        let mut caps = serde_json::Map::new();

        // Tauri app binary location
        let binary_path = Self::find_app_binary()?;

        // Set Tauri-specific capabilities
        caps.insert(
            "tauri:options".to_string(),
            json!({
                "application": binary_path.to_string_lossy().to_string(),
                "args": [
                    "--webdriver",  // Enable WebDriver mode
                ],
                "webviewOptions": {
                    "windowWidth": 1200,
                    "windowHeight": 800,
                },
                "env": {
                    "SOUL_PLAYER_DATA_DIR": app_data_dir.to_string_lossy().to_string(),
                    "SOUL_PLAYER_TEST_MODE": "1",
                }
            }),
        );

        Ok(caps)
    }

    /// Find the Soul Player app binary
    ///
    /// Searches in target directory for debug or release build
    fn find_app_binary() -> Result<PathBuf> {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("Invalid manifest directory")?
            .to_path_buf();

        // Try debug build first (most common during development)
        #[cfg(target_os = "windows")]
        let debug_path =
            workspace_root.join("applications/desktop/src-tauri/target/debug/soul-player.exe");
        #[cfg(target_os = "macos")]
        let debug_path = workspace_root.join("applications/desktop/src-tauri/target/debug/soul-player.app/Contents/MacOS/soul-player");
        #[cfg(target_os = "linux")]
        let debug_path =
            workspace_root.join("applications/desktop/src-tauri/target/debug/soul-player");

        if debug_path.exists() {
            return Ok(debug_path);
        }

        // Try release build
        #[cfg(target_os = "windows")]
        let release_path =
            workspace_root.join("applications/desktop/src-tauri/target/release/soul-player.exe");
        #[cfg(target_os = "macos")]
        let release_path = workspace_root.join("applications/desktop/src-tauri/target/release/soul-player.app/Contents/MacOS/soul-player");
        #[cfg(target_os = "linux")]
        let release_path =
            workspace_root.join("applications/desktop/src-tauri/target/release/soul-player");

        if release_path.exists() {
            return Ok(release_path);
        }

        anyhow::bail!(
            "Soul Player binary not found. Build the app first:\n  \
             cd applications/desktop && yarn dev:tauri --build"
        );
    }

    /// Wait for app window to be ready
    ///
    /// Waits for the main window to load and the app to initialize.
    pub async fn wait_for_window(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        loop {
            // Try to find a basic element that exists when app is ready
            if let Ok(_) = self.client.find(Locator::Css("body")).await {
                tracing::info!("[TauriDriver] App window ready");
                return Ok(());
            }

            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for app window to be ready");
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Load a test track into the player
    ///
    /// This injects a test track into the database and queue.
    /// For E2E tests, we need to use real audio files.
    pub async fn load_test_track<P: AsRef<Path>>(&self, audio_file_path: P) -> Result<()> {
        let path_str = audio_file_path.as_ref().to_string_lossy().to_string();

        tracing::info!("[TauriDriver] Loading test track: {}", path_str);

        // Execute JavaScript to load track via Tauri commands
        self.client
            .execute(
                &format!(
                    r#"
                    window.__TAURI__.core.invoke('load_test_track', {{
                        path: '{}'
                    }})
                    "#,
                    path_str.replace('\\', "\\\\")
                ),
                vec![],
            )
            .await
            .context("Failed to load test track")?;

        tracing::info!("[TauriDriver] Test track loaded successfully");
        Ok(())
    }

    /// Click the play button
    pub async fn click_play(&self) -> Result<()> {
        tracing::info!("[TauriDriver] Clicking play button");

        let play_button = self
            .wait_for_element(
                Locator::Css("[data-testid='play-button']"),
                Duration::from_secs(5),
            )
            .await
            .context("Play button not found")?;

        play_button
            .click()
            .await
            .context("Failed to click play button")?;

        Ok(())
    }

    /// Click the pause button
    pub async fn click_pause(&self) -> Result<()> {
        tracing::info!("[TauriDriver] Clicking pause button");

        let pause_button = self
            .wait_for_element(
                Locator::Css("[data-testid='pause-button']"),
                Duration::from_secs(5),
            )
            .await
            .context("Pause button not found")?;

        pause_button
            .click()
            .await
            .context("Failed to click pause button")?;

        Ok(())
    }

    /// Wait for an element to appear
    pub async fn wait_for_element(
        &self,
        locator: Locator<'_>,
        timeout: Duration,
    ) -> Result<fantoccini::elements::Element> {
        let start = Instant::now();

        loop {
            if let Ok(element) = self.client.find(locator.clone()).await {
                return Ok(element);
            }

            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for element: {:?}", locator);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get current playback state
    ///
    /// Returns a JSON object with playback information:
    /// - `isPlaying`: boolean
    /// - `currentTrack`: object with track info
    /// - `position`: number (seconds)
    pub async fn get_playback_state(&self) -> Result<Value> {
        let result = self
            .client
            .execute(
                r#"
                return window.__TAURI__.core.invoke('get_playback_state');
                "#,
                vec![],
            )
            .await
            .context("Failed to get playback state")?;

        Ok(result)
    }

    /// Wait for playback to start
    ///
    /// Polls the playback state until isPlaying becomes true.
    pub async fn wait_for_playback_start(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        loop {
            if let Ok(state) = self.get_playback_state().await {
                if let Some(is_playing) = state.get("isPlaying").and_then(|v| v.as_bool()) {
                    if is_playing {
                        tracing::info!("[TauriDriver] Playback started");
                        return Ok(());
                    }
                }
            }

            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for playback to start");
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get reference to underlying WebDriver client
    ///
    /// Use this for custom WebDriver operations not covered by helper methods.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Take screenshot
    ///
    /// Useful for debugging test failures.
    pub async fn screenshot<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let png_data = self
            .client
            .screenshot()
            .await
            .context("Failed to take screenshot")?;

        std::fs::write(output_path.as_ref(), png_data).context("Failed to write screenshot")?;

        tracing::info!(
            "[TauriDriver] Screenshot saved to: {}",
            output_path.as_ref().display()
        );

        Ok(())
    }
}

impl Drop for TauriDriver {
    fn drop(&mut self) {
        // Client cleanup is automatic via Drop
        // Driver process is killed when Child is dropped
        tracing::info!("[TauriDriver] Cleaning up WebDriver session");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run when explicitly requested (requires built app)
    async fn test_driver_launch() {
        tracing_subscriber::fmt::init();

        let driver = TauriDriver::new().await.expect("Failed to launch driver");

        driver
            .wait_for_window(Duration::from_secs(10))
            .await
            .expect("Window not ready");

        // App should be running at this point
        assert!(driver.client.current_url().await.is_ok());
    }
}
