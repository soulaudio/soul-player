//! Window state persistence
//!
//! This module manages window position, size, and route state across app launches.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::window_state;
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Load window state
//! let state = window_state::get_window_state(pool, "1").await?;
//!
//! // Save window state
//! window_state::save_window_state(pool, "1", &state).await?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::StorageError;

pub type Result<T> = std::result::Result<T, StorageError>;

/// Window state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    /// X position (None if not set)
    pub x: Option<i32>,
    /// Y position (None if not set)
    pub y: Option<i32>,
    /// Window width
    pub width: i32,
    /// Window height
    pub height: i32,
    /// Whether window is maximized
    pub maximized: bool,
    /// Last visited route (for restoration)
    pub last_route: Option<String>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1200,
            height: 800,
            maximized: false,
            last_route: Some("/".to_string()),
        }
    }
}

/// Get window state for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
///
/// # Returns
///
/// Returns the window state if it exists, otherwise returns default state
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn get_window_state(pool: &SqlitePool, user_id: &str) -> Result<WindowState> {
    let result = sqlx::query!(
        "SELECT x, y, width, height, maximized, last_route FROM window_state WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(result
        .map(|row| WindowState {
            x: row.x.map(|v| v as i32),
            y: row.y.map(|v| v as i32),
            width: row.width as i32,
            height: row.height as i32,
            maximized: row.maximized != 0,
            last_route: row.last_route,
        })
        .unwrap_or_default())
}

/// Save window state for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `state` - Window state to save
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn save_window_state(
    pool: &SqlitePool,
    user_id: &str,
    state: &WindowState,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let maximized: i64 = i64::from(state.maximized);
    let x = state.x.map(|v| v as i64);
    let y = state.y.map(|v| v as i64);
    let width = state.width as i64;
    let height = state.height as i64;

    sqlx::query!(
        "INSERT INTO window_state (user_id, x, y, width, height, maximized, last_route, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET
            x = ?, y = ?, width = ?, height = ?, maximized = ?, last_route = ?, updated_at = ?",
        user_id,
        x,
        y,
        width,
        height,
        maximized,
        state.last_route,
        now,
        x,
        y,
        width,
        height,
        maximized,
        state.last_route,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}
