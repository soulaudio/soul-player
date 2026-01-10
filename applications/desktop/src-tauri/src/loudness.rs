//! Loudness analysis Tauri commands
//!
//! Provides commands for analyzing audio tracks for loudness normalization
//! (ReplayGain 2.0 and EBU R128), managing analysis queue, and retrieving
//! loudness metadata.

use crate::app_state::AppState;
use crate::playback::PlaybackManager;
use serde::{Deserialize, Serialize};
use soul_loudness::{LoudnessAnalyzer, LoudnessInfo, NormalizationMode, ReplayGainCalculator};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::Mutex;

/// Loudness info for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLoudnessInfo {
    /// Track ID
    pub track_id: i64,
    /// ReplayGain track gain in dB
    pub replaygain_track_gain: Option<f64>,
    /// ReplayGain track peak (linear)
    pub replaygain_track_peak: Option<f64>,
    /// ReplayGain album gain in dB
    pub replaygain_album_gain: Option<f64>,
    /// ReplayGain album peak (linear)
    pub replaygain_album_peak: Option<f64>,
    /// Integrated loudness in LUFS
    pub lufs_integrated: Option<f64>,
    /// Loudness range in LU
    pub lufs_range: Option<f64>,
    /// True peak in dBFS
    pub true_peak_dbfs: Option<f64>,
    /// Whether the track has been analyzed
    pub is_analyzed: bool,
}

impl From<soul_storage::loudness::TrackLoudness> for FrontendLoudnessInfo {
    fn from(l: soul_storage::loudness::TrackLoudness) -> Self {
        Self {
            track_id: l.track_id,
            replaygain_track_gain: l.replaygain_track_gain,
            replaygain_track_peak: l.replaygain_track_peak,
            replaygain_album_gain: l.replaygain_album_gain,
            replaygain_album_peak: l.replaygain_album_peak,
            lufs_integrated: l.lufs_integrated,
            lufs_range: l.lufs_range,
            true_peak_dbfs: l.true_peak_dbfs,
            is_analyzed: l.is_analyzed(),
        }
    }
}

/// Analysis queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueStats {
    pub total: i32,
    pub pending: i32,
    pub processing: i32,
    pub completed: i32,
    pub failed: i32,
}

impl From<soul_storage::loudness::QueueStats> for QueueStats {
    fn from(s: soul_storage::loudness::QueueStats) -> Self {
        Self {
            total: s.total,
            pending: s.pending,
            processing: s.processing,
            completed: s.completed,
            failed: s.failed,
        }
    }
}

/// Background analysis worker state
pub struct AnalysisWorker {
    /// Whether the worker is running
    pub is_running: AtomicBool,
    /// Number of tracks analyzed in current session
    pub tracks_analyzed: AtomicUsize,
    /// Cancel flag
    pub cancel_requested: AtomicBool,
}

impl Default for AnalysisWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisWorker {
    pub fn new() -> Self {
        Self {
            is_running: AtomicBool::new(false),
            tracks_analyzed: AtomicUsize::new(0),
            cancel_requested: AtomicBool::new(false),
        }
    }
}

/// Loudness analysis version string
const ANALYSIS_VERSION: &str = "1.0.0-ebur128";

/// Get loudness information for a track
#[tauri::command]
pub async fn get_track_loudness(
    track_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendLoudnessInfo>, String> {
    let loudness = soul_storage::loudness::get_track_loudness(&state.pool, track_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(loudness.map(FrontendLoudnessInfo::from))
}

/// Analyze a single track and store results
#[tauri::command]
pub async fn analyze_track(
    track_id: i64,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<FrontendLoudnessInfo, String> {
    // Get track file path
    let track_id_str = soul_core::types::TrackId::new(track_id.to_string());
    let track = soul_storage::tracks::get_by_id(&state.pool, track_id_str)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track {} not found", track_id))?;

    // Find file path
    let file_path = track
        .availability
        .iter()
        .find_map(|avail| {
            if matches!(
                avail.status,
                soul_core::types::AvailabilityStatus::LocalFile
                    | soul_core::types::AvailabilityStatus::Cached
            ) {
                avail.local_file_path.clone()
            } else {
                None
            }
        })
        .ok_or_else(|| format!("No local file found for track {}", track_id))?;

    // Analyze the file
    let loudness_info = analyze_audio_file(&file_path).await?;

    // Calculate ReplayGain
    let rg_calculator = ReplayGainCalculator::new();
    let track_gain = rg_calculator.track_gain(&loudness_info);

    // Convert peak from dBFS to linear
    let peak_linear = if loudness_info.true_peak_dbfs > -100.0 {
        10.0_f64.powf(loudness_info.true_peak_dbfs / 20.0)
    } else {
        0.0
    };

    // Build storage struct
    let track_loudness = soul_storage::loudness::TrackLoudness {
        track_id,
        replaygain_track_gain: Some(track_gain.gain_db),
        replaygain_track_peak: Some(peak_linear),
        replaygain_album_gain: None,
        replaygain_album_peak: None,
        lufs_integrated: Some(loudness_info.integrated_lufs),
        lufs_range: Some(loudness_info.loudness_range_lu),
        true_peak_dbfs: Some(loudness_info.true_peak_dbfs),
        analyzed_at: None,
        version: None,
    };

    // Store results
    soul_storage::loudness::update_track_loudness(&state.pool, track_id, &track_loudness, ANALYSIS_VERSION)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event for UI update
    let _ = app.emit("loudness-analysis-complete", track_id);

    Ok(FrontendLoudnessInfo::from(track_loudness))
}

/// Queue a track for background analysis
#[tauri::command]
pub async fn queue_track_analysis(
    track_id: i64,
    priority: Option<i32>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    soul_storage::loudness::queue_track_for_analysis(&state.pool, track_id, priority.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

/// Queue all tracks without loudness data for analysis
#[tauri::command]
pub async fn queue_all_unanalyzed(state: State<'_, AppState>) -> Result<i64, String> {
    soul_storage::loudness::queue_all_unanalyzed(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get analysis queue statistics
#[tauri::command]
pub async fn get_analysis_queue_stats(state: State<'_, AppState>) -> Result<QueueStats, String> {
    let stats = soul_storage::loudness::get_queue_stats(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(QueueStats::from(stats))
}

/// Start background analysis worker
#[tauri::command]
pub async fn start_analysis_worker(
    state: State<'_, AppState>,
    worker_state: State<'_, Arc<Mutex<AnalysisWorker>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let worker = worker_state.lock().await;

    // Check if already running
    if worker.is_running.load(Ordering::SeqCst) {
        return Err("Analysis worker is already running".to_string());
    }

    // Set running flag
    worker.is_running.store(true, Ordering::SeqCst);
    worker.cancel_requested.store(false, Ordering::SeqCst);
    worker.tracks_analyzed.store(0, Ordering::SeqCst);
    drop(worker);

    // Clone what we need for the background task
    let pool = (*state.pool).clone();
    let worker_arc = (*worker_state).clone();

    // Spawn background analysis task
    tokio::spawn(async move {
        run_analysis_worker(pool, worker_arc, app).await;
    });

    Ok(())
}

/// Stop background analysis worker
#[tauri::command]
pub async fn stop_analysis_worker(
    worker_state: State<'_, Arc<Mutex<AnalysisWorker>>>,
) -> Result<(), String> {
    let worker = worker_state.lock().await;
    worker.cancel_requested.store(true, Ordering::SeqCst);
    Ok(())
}

/// Get analysis worker status
#[tauri::command]
pub async fn get_analysis_worker_status(
    worker_state: State<'_, Arc<Mutex<AnalysisWorker>>>,
) -> Result<serde_json::Value, String> {
    let worker = worker_state.lock().await;
    Ok(serde_json::json!({
        "isRunning": worker.is_running.load(Ordering::SeqCst),
        "tracksAnalyzed": worker.tracks_analyzed.load(Ordering::SeqCst),
    }))
}

/// Set volume leveling mode for playback
#[tauri::command]
pub async fn set_volume_leveling_mode(
    mode: String,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    let normalization_mode = match mode.as_str() {
        "disabled" => NormalizationMode::Disabled,
        "replaygain_track" => NormalizationMode::ReplayGainTrack,
        "replaygain_album" => NormalizationMode::ReplayGainAlbum,
        "ebu_r128" => NormalizationMode::EbuR128Broadcast,
        "streaming" => NormalizationMode::EbuR128Streaming,
        _ => return Err(format!("Invalid volume leveling mode: {}", mode)),
    };

    playback.set_volume_leveling_mode(normalization_mode);
    eprintln!("[set_volume_leveling_mode] Mode set to: {:?}", normalization_mode);
    Ok(())
}

/// Clear completed items from analysis queue
#[tauri::command]
pub async fn clear_completed_analysis(state: State<'_, AppState>) -> Result<i64, String> {
    soul_storage::loudness::clear_completed_queue_items(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

// Helper functions

/// Analyze an audio file and return loudness information
async fn analyze_audio_file(file_path: &str) -> Result<LoudnessInfo, String> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Use symphonia to decode and analyze
    let file_path = file_path.to_string();

    tokio::task::spawn_blocking(move || {
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        // Open file
        let file = std::fs::File::open(&file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Probe format
        let hint = Hint::new();
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| format!("Failed to probe format: {}", e))?;

        let mut format = probed.format;

        // Find audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No audio track found".to_string())?;

        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| "Unknown sample rate".to_string())?;
        let channels = track
            .codec_params
            .channels
            .ok_or_else(|| "Unknown channel count".to_string())?
            .count() as u32;

        // Create decoder
        let dec_opts = DecoderOptions::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        // Create loudness analyzer
        let mut analyzer = LoudnessAnalyzer::new(sample_rate, channels)
            .map_err(|e| format!("Failed to create analyzer: {}", e))?;

        // Decode and analyze
        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    eprintln!("[analyze_audio_file] Error reading packet: {}", e);
                    break;
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[analyze_audio_file] Decode error: {}", e);
                    continue;
                }
            };

            // Convert to f32 samples
            if sample_buf.is_none() {
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                sample_buf = Some(SampleBuffer::new(duration, spec));
            }

            let buf = sample_buf.as_mut().unwrap();
            buf.copy_interleaved_ref(decoded);

            // Add samples to analyzer
            if let Err(e) = analyzer.add_frames(buf.samples()) {
                eprintln!("[analyze_audio_file] Analysis error: {}", e);
            }
        }

        // Finalize analysis
        analyzer.finalize().map_err(|e| format!("Analysis failed: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Background analysis worker loop
async fn run_analysis_worker(
    pool: sqlx::SqlitePool,
    worker: Arc<Mutex<AnalysisWorker>>,
    app: tauri::AppHandle,
) {
    eprintln!("[analysis_worker] Starting background analysis");

    loop {
        // Check for cancellation
        {
            let w = worker.lock().await;
            if w.cancel_requested.load(Ordering::SeqCst) {
                eprintln!("[analysis_worker] Cancel requested, stopping");
                w.is_running.store(false, Ordering::SeqCst);
                let _ = app.emit("analysis-worker-stopped", ());
                return;
            }
        }

        // Get next item from queue
        let item = match soul_storage::loudness::get_next_queue_item(&pool).await {
            Ok(Some(item)) => item,
            Ok(None) => {
                eprintln!("[analysis_worker] Queue empty, stopping");
                let w = worker.lock().await;
                w.is_running.store(false, Ordering::SeqCst);
                let _ = app.emit("analysis-worker-complete", ());
                return;
            }
            Err(e) => {
                eprintln!("[analysis_worker] Error getting queue item: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        eprintln!("[analysis_worker] Processing track {}", item.track_id);

        // Mark as processing
        if let Err(e) = soul_storage::loudness::mark_queue_processing(&pool, item.id).await {
            eprintln!("[analysis_worker] Failed to mark processing: {}", e);
        }

        // Get track file path
        let track_id_str = soul_core::types::TrackId::new(item.track_id.to_string());
        let track = match soul_storage::tracks::get_by_id(&pool, track_id_str).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                let _ = soul_storage::loudness::mark_queue_failed(&pool, item.id, "Track not found").await;
                continue;
            }
            Err(e) => {
                let _ = soul_storage::loudness::mark_queue_failed(&pool, item.id, &e.to_string()).await;
                continue;
            }
        };

        // Find file path
        let file_path = track.availability.iter().find_map(|avail| {
            if matches!(
                avail.status,
                soul_core::types::AvailabilityStatus::LocalFile
                    | soul_core::types::AvailabilityStatus::Cached
            ) {
                avail.local_file_path.clone()
            } else {
                None
            }
        });

        let Some(file_path) = file_path else {
            let _ = soul_storage::loudness::mark_queue_failed(&pool, item.id, "No local file").await;
            continue;
        };

        // Analyze
        match analyze_audio_file(&file_path).await {
            Ok(loudness_info) => {
                // Calculate ReplayGain
                let rg_calculator = ReplayGainCalculator::new();
                let track_gain = rg_calculator.track_gain(&loudness_info);
                let peak_linear = if loudness_info.true_peak_dbfs > -100.0 {
                    10.0_f64.powf(loudness_info.true_peak_dbfs / 20.0)
                } else {
                    0.0
                };

                let track_loudness = soul_storage::loudness::TrackLoudness {
                    track_id: item.track_id,
                    replaygain_track_gain: Some(track_gain.gain_db),
                    replaygain_track_peak: Some(peak_linear),
                    replaygain_album_gain: None,
                    replaygain_album_peak: None,
                    lufs_integrated: Some(loudness_info.integrated_lufs),
                    lufs_range: Some(loudness_info.loudness_range_lu),
                    true_peak_dbfs: Some(loudness_info.true_peak_dbfs),
                    analyzed_at: None,
                    version: None,
                };

                // Store results
                if let Err(e) = soul_storage::loudness::update_track_loudness(
                    &pool,
                    item.track_id,
                    &track_loudness,
                    ANALYSIS_VERSION,
                )
                .await
                {
                    eprintln!("[analysis_worker] Failed to store results: {}", e);
                    let _ = soul_storage::loudness::mark_queue_failed(&pool, item.id, &e.to_string()).await;
                    continue;
                }

                // Mark completed
                let _ = soul_storage::loudness::mark_queue_completed(&pool, item.id).await;

                // Update counter
                {
                    let w = worker.lock().await;
                    w.tracks_analyzed.fetch_add(1, Ordering::SeqCst);
                }

                // Emit progress event
                let _ = app.emit("loudness-analysis-progress", serde_json::json!({
                    "trackId": item.track_id,
                    "trackTitle": track.title,
                    "lufsIntegrated": loudness_info.integrated_lufs,
                    "replaygainGain": track_gain.gain_db,
                }));

                eprintln!(
                    "[analysis_worker] Track {} analyzed: {:.1} LUFS, {:.2} dB gain",
                    item.track_id, loudness_info.integrated_lufs, track_gain.gain_db
                );
            }
            Err(e) => {
                eprintln!("[analysis_worker] Analysis failed for track {}: {}", item.track_id, e);
                let _ = soul_storage::loudness::mark_queue_failed(&pool, item.id, &e).await;
            }
        }

        // Small delay between tracks to avoid overloading
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
