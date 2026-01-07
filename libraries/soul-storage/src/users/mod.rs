//! User management and authentication queries

use crate::StorageError;
use soul_core::types::User;
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get user's password hash for authentication
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID to look up
///
/// # Returns
///
/// Returns the password hash if found, or None if user has no credentials
pub async fn get_password_hash(pool: &SqlitePool, user_id: &str) -> Result<Option<String>> {
    let row = sqlx::query!(
        "SELECT password_hash FROM user_credentials WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.password_hash))
}

/// Create or update user credentials
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `password_hash` - Hashed password (should already be hashed with bcrypt/argon2)
pub async fn set_password_hash(
    pool: &SqlitePool,
    user_id: &str,
    password_hash: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_credentials (user_id, password_hash, updated_at)
         VALUES (?, ?, datetime('now'))
         ON CONFLICT(user_id)
         DO UPDATE SET password_hash = excluded.password_hash, updated_at = datetime('now')",
        user_id,
        password_hash
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if user has credentials set
pub async fn has_credentials(pool: &SqlitePool, user_id: &str) -> Result<bool> {
    let row = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_credentials WHERE user_id = ?",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.count > 0)
}

/// Delete user credentials
pub async fn delete_credentials(pool: &SqlitePool, user_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM user_credentials WHERE user_id = ?", user_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get all users
pub async fn get_all(pool: &SqlitePool) -> Result<Vec<User>> {
    let rows = sqlx::query!("SELECT id, name, created_at FROM users ORDER BY name")
        .fetch_all(pool)
        .await?;

    let users = rows
        .into_iter()
        .filter_map(|row| {
            let id = row.id.parse::<i64>().ok()?;
            let created_at = chrono::DateTime::from_timestamp(row.created_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            Some(User {
                id,
                name: row.name,
                created_at,
            })
        })
        .collect();

    Ok(users)
}
