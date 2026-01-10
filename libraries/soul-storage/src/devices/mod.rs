//! Device management for multi-device sync

use crate::StorageError;
use soul_core::types::{Device, DeviceType};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Register a new device
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `id` - Device UUID
/// * `user_id` - Owner user ID
/// * `name` - Display name
/// * `device_type` - Platform type
pub async fn register(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    name: &str,
    device_type: DeviceType,
) -> Result<Device> {
    let now = chrono::Utc::now().timestamp();
    let device_type_str = device_type.as_str();

    sqlx::query!(
        "INSERT INTO devices (id, user_id, name, device_type, last_seen_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
        id,
        user_id,
        name,
        device_type_str,
        now,
        now
    )
    .execute(pool)
    .await?;

    Ok(Device {
        id: id.to_string(),
        user_id: user_id.to_string(),
        name: name.to_string(),
        device_type,
        last_seen_at: now,
        created_at: now,
    })
}

/// Get a device by ID
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Device>> {
    let row = sqlx::query!(
        "SELECT id, user_id, name, device_type, last_seen_at, created_at
         FROM devices WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Device {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        device_type: DeviceType::from_str(&r.device_type).unwrap_or(DeviceType::Web),
        last_seen_at: r.last_seen_at,
        created_at: r.created_at,
    }))
}

/// Get all devices for a user
pub async fn get_by_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<Device>> {
    let rows = sqlx::query!(
        "SELECT id, user_id, name, device_type, last_seen_at, created_at
         FROM devices WHERE user_id = ? ORDER BY last_seen_at DESC",
        user_id
    )
    .fetch_all(pool)
    .await?;

    let devices = rows
        .into_iter()
        .map(|r| Device {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            device_type: DeviceType::from_str(&r.device_type).unwrap_or(DeviceType::Web),
            last_seen_at: r.last_seen_at,
            created_at: r.created_at,
        })
        .collect();

    Ok(devices)
}

/// Update device's last seen timestamp
pub async fn update_last_seen(pool: &SqlitePool, id: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!("UPDATE devices SET last_seen_at = ? WHERE id = ?", now, id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Unregister (delete) a device
pub async fn unregister(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query!("DELETE FROM devices WHERE id = ?", id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete inactive devices older than the given threshold
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `threshold_seconds` - Devices not seen within this many seconds will be deleted
pub async fn cleanup_inactive(pool: &SqlitePool, threshold_seconds: i64) -> Result<u64> {
    let cutoff = chrono::Utc::now().timestamp() - threshold_seconds;

    let result = sqlx::query!("DELETE FROM devices WHERE last_seen_at < ?", cutoff)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
