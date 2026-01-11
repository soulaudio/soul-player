//! Loudness metadata storage
//!
//! Database operations for storing and retrieving loudness analysis results
//! including ReplayGain values and EBU R128 measurements.

use soul_core::error::Result;
use sqlx::SqlitePool;

/// Loudness metadata for a track
#[derive(Debug, Clone, Default)]
pub struct TrackLoudness {
    /// Track ID
    pub track_id: i64,
    /// ReplayGain track gain in dB
    pub replaygain_track_gain: Option<f64>,
    /// ReplayGain track peak (linear 0.0-1.0+)
    pub replaygain_track_peak: Option<f64>,
    /// ReplayGain album gain in dB
    pub replaygain_album_gain: Option<f64>,
    /// ReplayGain album peak (linear 0.0-1.0+)
    pub replaygain_album_peak: Option<f64>,
    /// Integrated loudness in LUFS
    pub lufs_integrated: Option<f64>,
    /// Loudness range in LU
    pub lufs_range: Option<f64>,
    /// True peak in dBFS
    pub true_peak_dbfs: Option<f64>,
    /// Analysis timestamp (Unix epoch)
    pub analyzed_at: Option<i64>,
    /// Analysis algorithm version
    pub version: Option<String>,
}

impl TrackLoudness {
    /// Check if track has been analyzed
    pub fn is_analyzed(&self) -> bool {
        self.lufs_integrated.is_some()
    }

    /// Check if track has ReplayGain tags
    pub fn has_replaygain(&self) -> bool {
        self.replaygain_track_gain.is_some()
    }
}

/// Album loudness metadata
#[derive(Debug, Clone)]
pub struct AlbumLoudness {
    /// Album ID
    pub album_id: i64,
    /// Album-level ReplayGain in dB
    pub replaygain_gain: Option<f64>,
    /// Album-level peak (linear)
    pub replaygain_peak: Option<f64>,
    /// Album integrated loudness in LUFS
    pub lufs_integrated: Option<f64>,
    /// Album loudness range in LU
    pub lufs_range: Option<f64>,
    /// Album max true peak in dBFS
    pub true_peak_dbfs: Option<f64>,
    /// Number of tracks analyzed
    pub track_count: i32,
    /// Analysis timestamp
    pub analyzed_at: i64,
    /// Analysis algorithm version
    pub version: String,
}

/// Analysis queue item status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl AnalysisStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "processing" => Self::Processing,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

/// Analysis queue item
#[derive(Debug, Clone)]
pub struct AnalysisQueueItem {
    pub id: i64,
    pub track_id: i64,
    pub priority: i32,
    pub status: AnalysisStatus,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

/// Get loudness metadata for a track
pub async fn get_track_loudness(pool: &SqlitePool, track_id: i64) -> Result<Option<TrackLoudness>> {
    let row = sqlx::query!(
        r#"
        SELECT
            id,
            replaygain_track_gain,
            replaygain_track_peak,
            replaygain_album_gain,
            replaygain_album_peak,
            lufs_integrated,
            lufs_range,
            true_peak_dbfs,
            loudness_analyzed_at,
            loudness_version
        FROM tracks
        WHERE id = ?
        "#,
        track_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| TrackLoudness {
        track_id: r.id,
        replaygain_track_gain: r.replaygain_track_gain,
        replaygain_track_peak: r.replaygain_track_peak,
        replaygain_album_gain: r.replaygain_album_gain,
        replaygain_album_peak: r.replaygain_album_peak,
        lufs_integrated: r.lufs_integrated,
        lufs_range: r.lufs_range,
        true_peak_dbfs: r.true_peak_dbfs,
        analyzed_at: r.loudness_analyzed_at,
        version: r.loudness_version,
    }))
}

/// Update loudness metadata for a track
pub async fn update_track_loudness(
    pool: &SqlitePool,
    track_id: i64,
    loudness: &TrackLoudness,
    version: &str,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE tracks SET
            replaygain_track_gain = ?,
            replaygain_track_peak = ?,
            lufs_integrated = ?,
            lufs_range = ?,
            true_peak_dbfs = ?,
            loudness_analyzed_at = ?,
            loudness_version = ?
        WHERE id = ?
        "#,
        loudness.replaygain_track_gain,
        loudness.replaygain_track_peak,
        loudness.lufs_integrated,
        loudness.lufs_range,
        loudness.true_peak_dbfs,
        now,
        version,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update album gain for all tracks in an album
pub async fn update_album_loudness(
    pool: &SqlitePool,
    album_id: i64,
    album_gain: f64,
    album_peak: f64,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks SET
            replaygain_album_gain = ?,
            replaygain_album_peak = ?
        WHERE album_id = ?
        "#,
        album_gain,
        album_peak,
        album_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get tracks without loudness analysis
pub async fn get_tracks_without_analysis(pool: &SqlitePool, limit: i32) -> Result<Vec<i64>> {
    let rows = sqlx::query!(
        r#"
        SELECT id
        FROM tracks
        WHERE loudness_analyzed_at IS NULL
        LIMIT ?
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.id).collect())
}

/// Get tracks in an album for album gain calculation
pub async fn get_album_track_ids(pool: &SqlitePool, album_id: i64) -> Result<Vec<i64>> {
    let rows = sqlx::query!(
        r#"
        SELECT id
        FROM tracks
        WHERE album_id = ?
        ORDER BY disc_number, track_number
        "#,
        album_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.id).collect())
}

/// Add track to analysis queue
pub async fn queue_track_for_analysis(
    pool: &SqlitePool,
    track_id: i64,
    priority: i32,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO loudness_analysis_queue (track_id, priority, status, created_at)
        VALUES (?, ?, 'pending', ?)
        ON CONFLICT(track_id) DO UPDATE SET
            priority = MAX(priority, excluded.priority),
            status = CASE WHEN status = 'failed' THEN 'pending' ELSE status END
        "#,
        track_id,
        priority,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get next item from analysis queue
pub async fn get_next_queue_item(pool: &SqlitePool) -> Result<Option<AnalysisQueueItem>> {
    let row = sqlx::query!(
        r#"
        SELECT id, track_id as "track_id: i64", priority, status, error_message, created_at, started_at, completed_at
        FROM loudness_analysis_queue
        WHERE status = 'pending'
        ORDER BY priority DESC, created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AnalysisQueueItem {
        id: r.id.unwrap_or(0),
        track_id: r.track_id,
        priority: r.priority.unwrap_or(0) as i32,
        status: AnalysisStatus::from_str(&r.status.unwrap_or_default()),
        error_message: r.error_message,
        created_at: r.created_at,
        started_at: r.started_at,
        completed_at: r.completed_at,
    }))
}

/// Mark queue item as processing
pub async fn mark_queue_processing(pool: &SqlitePool, queue_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE loudness_analysis_queue
        SET status = 'processing', started_at = ?
        WHERE id = ?
        "#,
        now,
        queue_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark queue item as completed
pub async fn mark_queue_completed(pool: &SqlitePool, queue_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE loudness_analysis_queue
        SET status = 'completed', completed_at = ?
        WHERE id = ?
        "#,
        now,
        queue_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark queue item as failed
pub async fn mark_queue_failed(pool: &SqlitePool, queue_id: i64, error: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE loudness_analysis_queue
        SET status = 'failed', completed_at = ?, error_message = ?
        WHERE id = ?
        "#,
        now,
        error,
        queue_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get queue statistics
pub async fn get_queue_stats(pool: &SqlitePool) -> Result<QueueStats> {
    let row = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total,
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
            SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END) as processing,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
        FROM loudness_analysis_queue
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(QueueStats {
        total: row.total as i32,
        pending: row.pending.unwrap_or(0) as i32,
        processing: row.processing.unwrap_or(0) as i32,
        completed: row.completed.unwrap_or(0) as i32,
        failed: row.failed.unwrap_or(0) as i32,
    })
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total: i32,
    pub pending: i32,
    pub processing: i32,
    pub completed: i32,
    pub failed: i32,
}

/// Clear completed items from queue
pub async fn clear_completed_queue_items(pool: &SqlitePool) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM loudness_analysis_queue
        WHERE status = 'completed'
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}

/// Queue all unanalyzed tracks for analysis
pub async fn queue_all_unanalyzed(pool: &SqlitePool) -> Result<i64> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query!(
        r#"
        INSERT INTO loudness_analysis_queue (track_id, priority, status, created_at)
        SELECT id, 0, 'pending', ?
        FROM tracks
        WHERE loudness_analyzed_at IS NULL
        ON CONFLICT(track_id) DO NOTHING
        "#,
        now
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}
