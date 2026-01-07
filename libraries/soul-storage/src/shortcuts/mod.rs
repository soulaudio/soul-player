//! Global keyboard shortcuts configuration
//!
//! This module manages user-customizable keyboard shortcuts for media controls.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::shortcuts;
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Get all shortcuts for a user
//! let shortcuts = shortcuts::get_shortcuts(pool, "1").await?;
//!
//! // Set a custom shortcut
//! shortcuts::set_shortcut(
//!     pool,
//!     "1",
//!     shortcuts::ShortcutAction::PlayPause,
//!     "CommandOrControl+P".to_string()
//! ).await?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::StorageError;

pub type Result<T> = std::result::Result<T, StorageError>;

/// Available shortcut actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    /// Play or pause playback
    PlayPause,
    /// Skip to next track
    Next,
    /// Skip to previous track
    Previous,
    /// Increase volume
    VolumeUp,
    /// Decrease volume
    VolumeDown,
    /// Mute audio
    Mute,
    /// Toggle shuffle mode
    ToggleShuffle,
    /// Toggle repeat mode
    ToggleRepeat,
}

impl ShortcutAction {
    /// Convert action to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlayPause => "play_pause",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::VolumeUp => "volume_up",
            Self::VolumeDown => "volume_down",
            Self::Mute => "mute",
            Self::ToggleShuffle => "toggle_shuffle",
            Self::ToggleRepeat => "toggle_repeat",
        }
    }

    /// Parse action from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "play_pause" => Some(Self::PlayPause),
            "next" => Some(Self::Next),
            "previous" => Some(Self::Previous),
            "volume_up" => Some(Self::VolumeUp),
            "volume_down" => Some(Self::VolumeDown),
            "mute" => Some(Self::Mute),
            "toggle_shuffle" => Some(Self::ToggleShuffle),
            "toggle_repeat" => Some(Self::ToggleRepeat),
            _ => None,
        }
    }
}

/// Global keyboard shortcut configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalShortcut {
    /// Action triggered by this shortcut
    pub action: ShortcutAction,
    /// Keyboard accelerator (e.g., `MediaPlayPause`, `CommandOrControl+P`)
    pub accelerator: String,
    /// Whether this shortcut is enabled
    pub enabled: bool,
    /// Whether this is a default (built-in) shortcut
    pub is_default: bool,
}

/// Get default keyboard shortcuts
///
/// These are the built-in shortcuts using `CommandOrControl` modifier
/// (Command on macOS, Control on Windows/Linux)
pub fn default_shortcuts() -> Vec<GlobalShortcut> {
    vec![
        GlobalShortcut {
            action: ShortcutAction::PlayPause,
            accelerator: "CommandOrControl+Space".to_string(),
            enabled: true,
            is_default: true,
        },
        GlobalShortcut {
            action: ShortcutAction::Next,
            accelerator: "CommandOrControl+Right".to_string(),
            enabled: true,
            is_default: true,
        },
        GlobalShortcut {
            action: ShortcutAction::Previous,
            accelerator: "CommandOrControl+Left".to_string(),
            enabled: true,
            is_default: true,
        },
        GlobalShortcut {
            action: ShortcutAction::VolumeUp,
            accelerator: "CommandOrControl+Up".to_string(),
            enabled: true,
            is_default: true,
        },
        GlobalShortcut {
            action: ShortcutAction::VolumeDown,
            accelerator: "CommandOrControl+Down".to_string(),
            enabled: true,
            is_default: true,
        },
        GlobalShortcut {
            action: ShortcutAction::Mute,
            accelerator: "CommandOrControl+M".to_string(),
            enabled: true,
            is_default: true,
        },
    ]
}

/// Get all keyboard shortcuts for a user
///
/// If no shortcuts are configured, initializes with defaults
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
///
/// # Returns
///
/// Returns a vector of all configured shortcuts
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn get_shortcuts(pool: &SqlitePool, user_id: &str) -> Result<Vec<GlobalShortcut>> {
    let rows = sqlx::query!(
        "SELECT action, accelerator, enabled, is_default FROM global_shortcuts WHERE user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        // Initialize with defaults
        let defaults = default_shortcuts();
        for sc in &defaults {
            let _ = set_shortcut(pool, user_id, sc.action.clone(), sc.accelerator.clone()).await;
        }
        return Ok(defaults);
    }

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some(GlobalShortcut {
                action: ShortcutAction::from_str(&row.action)?,
                accelerator: row.accelerator,
                enabled: row.enabled != 0,
                is_default: row.is_default != 0,
            })
        })
        .collect())
}

/// Set a keyboard shortcut for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `action` - Shortcut action
/// * `accelerator` - Keyboard accelerator string
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn set_shortcut(
    pool: &SqlitePool,
    user_id: &str,
    action: ShortcutAction,
    accelerator: String,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let action_str = action.as_str();

    sqlx::query!(
        "INSERT INTO global_shortcuts (user_id, action, accelerator, enabled, is_default, updated_at)
         VALUES (?, ?, ?, 1, 0, ?)
         ON CONFLICT(user_id, action) DO UPDATE SET accelerator = excluded.accelerator, updated_at = excluded.updated_at",
        user_id,
        action_str,
        accelerator,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Reset all shortcuts to defaults for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn reset_shortcuts_to_defaults(pool: &SqlitePool, user_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM global_shortcuts WHERE user_id = ?", user_id)
        .execute(pool)
        .await?;

    for sc in default_shortcuts() {
        set_shortcut(pool, user_id, sc.action, sc.accelerator).await?;
    }

    Ok(())
}
