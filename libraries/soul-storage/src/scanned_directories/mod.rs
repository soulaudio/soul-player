//! Scanned directory tracking storage
//!
//! Stores directory-level mtime information for incremental scanning.
//! By tracking the last-known mtime of each directory, unchanged
//! directories can be skipped entirely during rescan.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::scanned_directories;
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Upsert a batch of scanned directories
//! let dirs = vec![
//!     ("/music/album1".to_string(), 1710000000_i64, 12_i64),
//!     ("/music/album2".to_string(), 1710000100_i64, 8_i64),
//! ];
//! scanned_directories::upsert_batch(pool, 1, &dirs).await?;
//!
//! // Retrieve all scanned directories for a source
//! let entries = scanned_directories::get_by_source(pool, 1).await?;
//! # Ok(())
//! # }
//! ```

use crate::utils::time::now_timestamp;
use crate::StorageError;
use sqlx::{Row, SqlitePool};

type Result<T> = std::result::Result<T, StorageError>;

/// Row returned from the scanned_directories table
#[derive(Debug, Clone)]
pub struct ScannedDirectory {
    pub id: i64,
    pub library_source_id: i64,
    pub path: String,
    pub dir_mtime: i64,
    pub file_count: i64,
    pub last_scanned_at: i64,
}

/// Get all scanned directories for a library source
pub async fn get_by_source(pool: &SqlitePool, source_id: i64) -> Result<Vec<ScannedDirectory>> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, library_source_id, path, dir_mtime, file_count, last_scanned_at
         FROM scanned_directories
         WHERE library_source_id = ?",
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for r in rows {
        result.push(ScannedDirectory {
            id: r.get("id"),
            library_source_id: r.get("library_source_id"),
            path: r.get("path"),
            dir_mtime: r.get("dir_mtime"),
            file_count: r.get("file_count"),
            last_scanned_at: r.get("last_scanned_at"),
        });
    }
    Ok(result)
}

/// Upsert a batch of scanned directory records.
///
/// Each tuple is `(path, dir_mtime, file_count)`. Uses `INSERT OR REPLACE`
/// with the UNIQUE constraint on `(library_source_id, path)`.
/// Sets `last_scanned_at` to the current unix timestamp.
pub async fn upsert_batch(
    pool: &SqlitePool,
    source_id: i64,
    dirs: &[(String, i64, i64)],
) -> Result<()> {
    let now = now_timestamp();

    for (path, dir_mtime, file_count) in dirs {
        sqlx::query(
            "INSERT OR REPLACE INTO scanned_directories
                (library_source_id, path, dir_mtime, file_count, last_scanned_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(source_id)
        .bind(path)
        .bind(dir_mtime)
        .bind(file_count)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Delete all scanned directory records for a source.
///
/// This is useful when a full rescan is requested. Note that
/// `ON DELETE CASCADE` on the foreign key also handles cleanup
/// when a library source is deleted.
pub async fn delete_by_source(pool: &SqlitePool, source_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM scanned_directories WHERE library_source_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    Ok(())
}
