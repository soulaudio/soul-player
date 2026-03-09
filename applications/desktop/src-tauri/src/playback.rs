//! Playback management for Tauri desktop application
//!
//! This module wraps the DesktopPlayback system and provides
//! a clean interface for Tauri commands and event emission.

use serde::Serialize;
use soul_audio_desktop::{
    create_async_device_monitor, AudioError, DesktopPlayback, DeviceEvent, DeviceSwitchReason,
    ExclusiveConfig, LatencyInfo, PlaybackCommand, PlaybackEvent, Receiver,
};
use soul_playback::{
    lazy_queue::QueueContext, PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::app_state::AppState;
use crate::playback_constants::PlaybackTimingConfig;

/// Device event type for deduplication tracking
#[derive(Debug, Clone, PartialEq, Eq)]
enum DeviceEventType {
    Added,
    Removed,
    DefaultChanged,
    PropertyChanged,
}

/// Last device event tracker for deduplication
///
/// Platform APIs (CoreAudio, PipeWire, WinRT) can emit duplicate events like:
/// - Device removed twice
/// - Default device changed to same device multiple times
///
/// This tracker prevents redundant operations by filtering duplicates within the configured window.
struct LastDeviceEvent {
    event_type: DeviceEventType,
    device_id: String,
    timestamp: Instant,
}

impl LastDeviceEvent {
    /// Check if a new event is a duplicate of this event
    ///
    /// Returns true if:
    /// - Same event type
    /// - Same device ID
    /// - Within the deduplication window
    fn is_duplicate(&self, event_type: &DeviceEventType, device_id: &str) -> bool {
        if self.event_type != *event_type {
            return false;
        }
        if self.device_id != device_id {
            return false;
        }
        // Check if within deduplication window (using default timing config)
        let config = PlaybackTimingConfig::default();
        self.timestamp.elapsed() < config.device_dedup_duration()
    }
}

/// Device monitoring metrics for observability
///
/// Thread-safe atomic counters for tracking device events and switches.
/// All counters use Relaxed ordering since we only need eventual consistency
/// for metrics reporting.
#[derive(Debug, Default)]
struct DeviceMetrics {
    /// Total number of device switch attempts
    device_switches_total: AtomicU64,
    /// Number of successful device switches
    device_switches_successful: AtomicU64,
    /// Number of failed device switches
    device_switches_failed: AtomicU64,
    /// Number of device added events
    device_added_events: AtomicU64,
    /// Number of device removed events
    device_removed_events: AtomicU64,
    /// Number of default device changed events
    default_changed_events: AtomicU64,
    /// Number of device property changed events
    property_changed_events: AtomicU64,
    /// Unix timestamp (milliseconds) of last device switch
    last_switch_timestamp: AtomicU64,
    /// Duration (milliseconds) of last device switch
    last_switch_duration_ms: AtomicU64,
}

impl DeviceMetrics {
    /// Create new device metrics tracker
    fn new() -> Self {
        Self::default()
    }

    /// Record a device switch attempt start
    fn record_switch_start(&self) {
        self.device_switches_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful device switch with timing
    fn record_switch_success(&self, duration_ms: u64) {
        self.device_switches_successful
            .fetch_add(1, Ordering::Relaxed);
        self.last_switch_duration_ms
            .store(duration_ms, Ordering::Relaxed);

        // Store current Unix timestamp in milliseconds
        if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
            self.last_switch_timestamp
                .store(timestamp.as_millis() as u64, Ordering::Relaxed);
        }
    }

    /// Record a failed device switch
    fn record_switch_failure(&self) {
        self.device_switches_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a device added event
    fn record_device_added(&self) {
        self.device_added_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a device removed event
    fn record_device_removed(&self) {
        self.device_removed_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a default device changed event
    fn record_default_changed(&self) {
        self.default_changed_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a device property changed event
    fn record_property_changed(&self) {
        self.property_changed_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of current metrics
    fn snapshot(&self) -> DeviceMetricsSnapshot {
        DeviceMetricsSnapshot {
            device_switches_total: self.device_switches_total.load(Ordering::Relaxed),
            device_switches_successful: self.device_switches_successful.load(Ordering::Relaxed),
            device_switches_failed: self.device_switches_failed.load(Ordering::Relaxed),
            device_added_events: self.device_added_events.load(Ordering::Relaxed),
            device_removed_events: self.device_removed_events.load(Ordering::Relaxed),
            default_changed_events: self.default_changed_events.load(Ordering::Relaxed),
            property_changed_events: self.property_changed_events.load(Ordering::Relaxed),
            last_switch_timestamp: self.last_switch_timestamp.load(Ordering::Relaxed),
            last_switch_duration_ms: self.last_switch_duration_ms.load(Ordering::Relaxed),
        }
    }

    /// Log current metrics summary
    fn log_summary(&self) {
        let snapshot = self.snapshot();
        tracing::info!(
            device_switches_total = snapshot.device_switches_total,
            device_switches_successful = snapshot.device_switches_successful,
            device_switches_failed = snapshot.device_switches_failed,
            device_added_events = snapshot.device_added_events,
            device_removed_events = snapshot.device_removed_events,
            default_changed_events = snapshot.default_changed_events,
            property_changed_events = snapshot.property_changed_events,
            last_switch_duration_ms = snapshot.last_switch_duration_ms,
            "[DEVICE_METRICS] Device monitoring metrics summary"
        );
    }
}

/// Snapshot of device metrics for serialization
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMetricsSnapshot {
    device_switches_total: u64,
    device_switches_successful: u64,
    device_switches_failed: u64,
    device_added_events: u64,
    device_removed_events: u64,
    default_changed_events: u64,
    property_changed_events: u64,
    /// Unix timestamp in milliseconds
    last_switch_timestamp: u64,
    /// Duration in milliseconds
    last_switch_duration_ms: u64,
}
/// Playback tracker for recording play statistics
///
/// Tracks the current playing track to record plays to the database when:
/// - Track changes (previous track may be completed or skipped)
/// - Track finishes naturally (always completed)
/// - Playback stops mid-track (skipped)
struct PlaybackTracker {
    current_track_id: Option<String>,
    current_track_duration: Option<Duration>,
    playback_start_time: Option<std::time::Instant>,
    playback_start_position: Duration,
}

impl PlaybackTracker {
    fn new() -> Self {
        Self {
            current_track_id: None,
            current_track_duration: None,
            playback_start_time: None,
            playback_start_position: Duration::ZERO,
        }
    }

    /// Start tracking a new track
    fn start_tracking(&mut self, track_id: String, duration: Duration) {
        self.current_track_id = Some(track_id);
        self.current_track_duration = Some(duration);
        self.playback_start_time = Some(std::time::Instant::now());
        self.playback_start_position = Duration::ZERO;
    }

    /// Update the playback position after a seek
    fn update_position(&mut self, position: Duration) {
        self.playback_start_position = position;
        self.playback_start_time = Some(std::time::Instant::now());
    }

    /// Calculate the current playback position
    fn current_position(&self) -> Duration {
        match (self.playback_start_time, self.playback_start_position) {
            (Some(start_time), start_pos) => {
                let elapsed = start_time.elapsed();
                start_pos + elapsed
            }
            _ => Duration::ZERO,
        }
    }

    /// Calculate completion percentage (0.0 to 1.0+)
    fn calculate_completion_percentage(&self) -> f64 {
        match self.current_track_duration {
            Some(duration) if duration.as_secs_f64() > 0.0 => {
                let position = self.current_position();
                position.as_secs_f64() / duration.as_secs_f64()
            }
            _ => 0.0,
        }
    }

    /// Check if the track was completed (80% threshold)
    fn is_completed(&self) -> bool {
        self.calculate_completion_percentage() >= 0.8
    }

    /// Reset tracker (call after recording a play)
    fn reset(&mut self) {
        self.current_track_id = None;
        self.current_track_duration = None;
        self.playback_start_time = None;
        self.playback_start_position = Duration::ZERO;
    }
}

/// Track info for frontend events (with duration in seconds)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendTrackEvent {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration: f64, // seconds
    cover_art_path: Option<String>,
}

impl From<&QueueTrack> for FrontendTrackEvent {
    fn from(track: &QueueTrack) -> Self {
        use soul_playback::TrackSource;

        // Prefer album artwork if available (to pick up custom artwork)
        // Otherwise fall back to track artwork
        // Optimized: Use String::with_capacity + write! to reduce allocations in hot path
        let cover_art_path = if let TrackSource::Album { id, .. } = &track.source {
            use std::fmt::Write;
            let mut s = String::with_capacity(16 + id.len());
            write!(&mut s, "artwork://album/{}", id).unwrap();
            Some(s)
        } else {
            use std::fmt::Write;
            let mut s = String::with_capacity(16 + track.id.len());
            write!(&mut s, "artwork://track/{}", track.id).unwrap();
            Some(s)
        };

        Self {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            duration: track.duration.as_secs_f64(),
            cover_art_path,
        }
    }
}

/// Playback manager for Tauri application
///
/// Wraps DesktopPlayback and handles event emission to frontend.
pub struct PlaybackManager {
    playback: Arc<Mutex<DesktopPlayback>>,
    app_handle: AppHandle,
    #[cfg(feature = "effects")]
    effect_slots: Arc<Mutex<[Option<crate::dsp_commands::EffectSlotState>; 4]>>,
    /// Device monitor cancellation token (for graceful shutdown)
    device_monitor_cancel_token: CancellationToken,
    /// Device metrics for monitoring
    device_metrics: Arc<DeviceMetrics>,
}

impl PlaybackManager {
    /// Create a new playback manager
    pub fn new(app_handle: AppHandle) -> Result<Self, AudioError> {
        // Create playback config
        let config = PlaybackConfig::default();

        // Create desktop playback system
        let playback = DesktopPlayback::new(config)?;

        // Clone event receiver before wrapping in mutex to avoid mutex contention in event loop
        let event_rx = playback.clone_event_receiver();

        let playback = Arc::new(Mutex::new(playback));

        // Start event emission thread with its own event receiver
        // This eliminates mutex contention that was causing command delays
        {
            let playback_clone = Arc::clone(&playback);
            let app_handle_clone = app_handle.clone();

            thread::spawn(move || {
                Self::event_emission_loop(playback_clone, event_rx, app_handle_clone);
            });
        }

        // Create device metrics tracker
        let device_metrics = Arc::new(DeviceMetrics::new());

        // Create cancellation token for device monitoring
        let cancel_token = CancellationToken::new();

        // Start async device monitoring task
        // This provides real-time hotplug notifications on platforms that support it
        // (macOS CoreAudio property listeners, Linux PipeWire registry events, Windows WinRT DeviceWatcher)
        {
            let playback_clone = Arc::clone(&playback);
            let app_handle_clone = app_handle.clone();
            let metrics_clone = Arc::clone(&device_metrics);
            let cancel_token_clone = cancel_token.clone();

            let monitor_handle = tauri::async_runtime::spawn(async move {
                Self::device_monitoring_task(
                    playback_clone,
                    app_handle_clone,
                    metrics_clone,
                    cancel_token_clone,
                )
                .await;
            });

            // Log errors from device monitoring (runs for app lifetime)
            tauri::async_runtime::spawn(async move {
                if let Err(e) = monitor_handle.await {
                    tracing::error!("[DEVICE_MONITOR] Device monitoring task panicked: {:?}", e);
                }
            });
        }

        Ok(Self {
            playback,
            app_handle,
            #[cfg(feature = "effects")]
            effect_slots: Arc::new(Mutex::new([None, None, None, None])),
            device_monitor_cancel_token: cancel_token,
            device_metrics,
        })
    }

    /// Event emission loop that runs in background thread
    ///
    /// Uses channel-based blocking with timeout instead of busy-waiting.
    /// Wakes up immediately when events arrive, or when periodic task is due (position update interval).
    /// This significantly reduces CPU usage and power consumption compared to fixed 50ms polling.
    ///
    /// NOTE: Mutex locks use expect() with clear messages instead of unwrap()
    /// to aid debugging if the mutex is poisoned (indicates a panic in another thread).
    fn event_emission_loop(
        playback: Arc<Mutex<DesktopPlayback>>,
        event_rx: Receiver<PlaybackEvent>,
        app_handle: AppHandle,
    ) {
        let timing_config = PlaybackTimingConfig::default();
        let position_update_interval = timing_config.position_update_duration();

        let mut last_position_emit = std::time::Instant::now();
        let mut last_crossfade_progress_emit = std::time::Instant::now();
        let mut tracker = PlaybackTracker::new();

        loop {
            // Calculate time until next periodic task
            let time_until_position =
                position_update_interval.saturating_sub(last_position_emit.elapsed());

            // Wait for event with timeout = next periodic task (or 1ms minimum)
            let timeout = time_until_position.max(Duration::from_millis(1));

            // Block until event arrives or timeout expires WITHOUT holding mutex
            // This eliminates mutex contention that was causing command delays
            let event = event_rx.recv_timeout(timeout).ok();

            if let Some(event) = event {
                // Emit to frontend
                let _ = match &event {
                    PlaybackEvent::StateChanged(state) => {
                        // Record play if stopping mid-track
                        if matches!(state, soul_playback::PlaybackState::Stopped) {
                            if let Some(ref track_id) = tracker.current_track_id {
                                let completed = tracker.is_completed();
                                let duration_secs =
                                    tracker.current_position().as_secs_f64().max(0.0);

                                tracing::debug!(
                                    track_id = %track_id,
                                    completed = completed,
                                    duration_secs = duration_secs,
                                    "Recording play on stop"
                                );

                                Self::record_play_event(
                                    &app_handle,
                                    track_id.clone(),
                                    duration_secs,
                                    completed,
                                );

                                tracker.reset();
                            }
                        }

                        app_handle.emit("playback:state-changed", state)
                    }
                    PlaybackEvent::TrackChanged(track) => {
                        // BEFORE emitting to frontend, record previous track play
                        if let Some(ref prev_track_id) = tracker.current_track_id {
                            let completed = tracker.is_completed();
                            let duration_secs = tracker.current_position().as_secs_f64().max(0.0);

                            tracing::debug!(
                                track_id = %prev_track_id,
                                completed = completed,
                                duration_secs = duration_secs,
                                completion_pct = tracker.calculate_completion_percentage() * 100.0,
                                "Recording play on track change"
                            );

                            Self::record_play_event(
                                &app_handle,
                                prev_track_id.clone(),
                                duration_secs,
                                completed,
                            );
                        }

                        // Start tracking new track
                        if let Some(ref new_track) = track {
                            tracker.start_tracking(new_track.id.clone(), new_track.duration);
                            tracing::debug!(
                                track_id = %new_track.id,
                                duration_secs = new_track.duration.as_secs_f64(),
                                "Started tracking new track"
                            );
                        } else {
                            tracker.reset();
                        }

                        // Convert QueueTrack to FrontendTrackEvent with duration in seconds
                        let frontend_track = track.as_ref().map(FrontendTrackEvent::from);
                        if let Some(ref t) = frontend_track {
                            tracing::debug!(
                                track_id = %t.id,
                                title = %t.title,
                                cover_art_path = ?t.cover_art_path,
                                "Track changed"
                            );
                        } else {
                            tracing::debug!("Track changed: None");
                        }
                        app_handle.emit("playback:track-changed", frontend_track)
                    }
                    PlaybackEvent::PositionUpdated(position) => {
                        app_handle.emit("playback:position-updated", position)
                    }
                    PlaybackEvent::VolumeChanged(volume) => {
                        app_handle.emit("playback:volume-changed", volume)
                    }
                    PlaybackEvent::QueueUpdated => app_handle.emit("playback:queue-updated", ()),
                    PlaybackEvent::Error(error) => app_handle.emit("playback:error", error),
                    PlaybackEvent::SampleRateChanged(from, to) => {
                        tracing::debug!(from = from, to = to, "Sample rate changed");
                        app_handle.emit(
                            "playback:sample-rate-changed",
                            serde_json::json!({
                                "from": from,
                                "to": to
                            }),
                        )
                    }
                    PlaybackEvent::CrossfadeStarted {
                        from_track_id,
                        to_track_id,
                        duration_ms,
                    } => {
                        tracing::debug!(
                            from_track_id = %from_track_id,
                            to_track_id = %to_track_id,
                            duration_ms = duration_ms,
                            "Crossfade started"
                        );
                        app_handle.emit(
                            "playback:crossfade-started",
                            serde_json::json!({
                                "from_track_id": from_track_id,
                                "to_track_id": to_track_id,
                                "duration_ms": duration_ms
                            }),
                        )
                    }
                    PlaybackEvent::CrossfadeProgress {
                        progress,
                        metadata_switched,
                    } => {
                        // Throttle to max 20 updates/second (50ms minimum interval)
                        // This prevents event flooding during transitions while still providing smooth updates
                        if last_crossfade_progress_emit.elapsed() >= Duration::from_millis(50) {
                            last_crossfade_progress_emit = std::time::Instant::now();
                            app_handle.emit(
                                "playback:crossfade-progress",
                                serde_json::json!({
                                    "progress": progress,
                                    "metadata_switched": metadata_switched
                                }),
                            )
                        } else {
                            // Skip emission - too soon after last one
                            continue;
                        }
                    }
                    PlaybackEvent::CrossfadeCompleted => {
                        tracing::debug!("Crossfade completed");
                        app_handle.emit("playback:crossfade-completed", ())
                    }
                    PlaybackEvent::BatchLoadRequested { offset, limit } => {
                        tracing::debug!(offset = offset, limit = limit, "Batch load requested");

                        // Extract values before spawning async task (for 'static lifetime)
                        let offset_val = *offset;
                        let limit_val = *limit;

                        // Spawn async task to load batch (non-blocking)
                        let playback_clone = Arc::clone(&playback);
                        let app_handle_clone = app_handle.clone();

                        let batch_handle = tauri::async_runtime::spawn(async move {
                            Self::handle_batch_request(
                                playback_clone,
                                app_handle_clone,
                                offset_val,
                                limit_val,
                                false,
                            )
                            .await;
                        });

                        // Detach error logging to avoid blocking
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = batch_handle.await {
                                tracing::error!("[PLAYBACK] Batch load task panicked: {:?}", e);
                            }
                        });

                        // Don't emit to frontend - this is an internal event
                        continue;
                    }
                    PlaybackEvent::JumpLoadRequested { offset, limit } => {
                        tracing::debug!(offset = offset, limit = limit, "Jump load requested");

                        // Extract values before spawning async task (for 'static lifetime)
                        let offset_val = *offset;
                        let limit_val = *limit;

                        // Spawn async task to load batch (non-blocking)
                        let playback_clone = Arc::clone(&playback);
                        let app_handle_clone = app_handle.clone();

                        let jump_handle = tauri::async_runtime::spawn(async move {
                            Self::handle_batch_request(
                                playback_clone,
                                app_handle_clone,
                                offset_val,
                                limit_val,
                                true,
                            )
                            .await;
                        });

                        // Detach error logging to avoid blocking
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = jump_handle.await {
                                tracing::error!("[PLAYBACK] Jump load task panicked: {:?}", e);
                            }
                        });

                        // Don't emit to frontend - this is an internal event
                        continue;
                    }
                    PlaybackEvent::DeviceSwitchStarted {
                        target_device,
                        reason,
                    } => {
                        tracing::info!(
                            target_device = %target_device,
                            reason = %reason,
                            "[PLAYBACK] Device switch started"
                        );
                        app_handle.emit(
                            "audio:device-switch-started",
                            serde_json::json!({
                                "target_device": target_device,
                                "reason": reason.to_string()
                            }),
                        )
                    }
                    PlaybackEvent::DeviceSwitchCompleted {
                        device_name,
                        sample_rate,
                    } => {
                        tracing::info!(
                            device_name = %device_name,
                            sample_rate = sample_rate,
                            "[PLAYBACK] Device switch completed"
                        );
                        app_handle.emit(
                            "audio:device-switch-completed",
                            serde_json::json!({
                                "device_name": device_name,
                                "sample_rate": sample_rate
                            }),
                        )
                    }
                    PlaybackEvent::DeviceSwitchFailed {
                        error,
                        fallback_attempted,
                    } => {
                        tracing::error!(
                            error = %error,
                            fallback_attempted = fallback_attempted,
                            "[PLAYBACK] Device switch failed"
                        );
                        app_handle.emit(
                            "audio:device-switch-failed",
                            serde_json::json!({
                                "error": error,
                                "fallback_attempted": fallback_attempted
                            }),
                        )
                    }
                };
            }

            // Emit position updates at configured interval during playback
            if last_position_emit.elapsed() >= position_update_interval {
                match playback.lock() {
                    Ok(pb) => {
                        let position = pb.get_position();
                        let state = pb.get_state();
                        drop(pb);

                        if state == soul_playback::PlaybackState::Playing {
                            let _ = app_handle
                                .emit("playback:position-updated", position.as_secs_f64());
                        }

                        last_position_emit = std::time::Instant::now();
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "[playback] Failed to lock mutex for position update - skipping"
                        );
                        // Continue without updating position this iteration
                    }
                }
            }

            // No explicit sleep needed - recv_event_timeout() blocks efficiently
            // The loop continues immediately after timeout or event arrival
        }
    }

    /// Handle a single device event with deduplication
    ///
    /// This processes device events sequentially to prevent race conditions.
    /// Called by the event processing task from the bounded channel.
    ///
    /// Deduplication prevents redundant operations from duplicate platform events like:
    /// - Device removed twice
    /// - Default device changed to same device multiple times
    async fn handle_device_event(
        event: DeviceEvent,
        playback: &Arc<Mutex<DesktopPlayback>>,
        app_handle: &AppHandle,
        metrics: &Arc<DeviceMetrics>,
        last_event: &mut Option<LastDeviceEvent>,
    ) {
        // Extract event type and device ID for deduplication check
        let (event_type, device_id) = match &event {
            DeviceEvent::DeviceAdded { id, .. } => (DeviceEventType::Added, id.clone()),
            DeviceEvent::DeviceRemoved { id } => (DeviceEventType::Removed, id.clone()),
            DeviceEvent::DefaultDeviceChanged { id, .. } => {
                (DeviceEventType::DefaultChanged, id.clone())
            }
            DeviceEvent::DevicePropertyChanged { id, .. } => {
                (DeviceEventType::PropertyChanged, id.clone())
            }
        };

        // Check for duplicate event
        if let Some(ref prev) = last_event {
            if prev.is_duplicate(&event_type, &device_id) {
                tracing::debug!(
                    event_type = ?event_type,
                    device_id = %device_id,
                    elapsed_ms = prev.timestamp.elapsed().as_millis(),
                    "[DEVICE_MONITOR] Skipping duplicate event (within deduplication window)"
                );
                return;
            }
        }

        // Process the event (not a duplicate)
        match event {
            DeviceEvent::DeviceAdded { ref id, ref name } => {
                metrics.record_device_added();

                tracing::info!(
                    device_id = %id,
                    device_name = %name,
                    "[DEVICE_MONITOR] Device added"
                );

                // Emit to frontend (no blocking operations)
                if let Err(e) = app_handle.emit(
                    "audio:device-added",
                    serde_json::json!({
                        "id": id,
                        "name": name,
                    }),
                ) {
                    tracing::warn!(
                        error = %e,
                        event = "audio:device-added",
                        "[DEVICE_MONITOR] Failed to emit device event to frontend"
                    );
                }
            }
            DeviceEvent::DeviceRemoved { id, .. } => {
                metrics.record_device_removed();

                tracing::info!(
                    device_id = %id,
                    "[DEVICE_MONITOR] Device removed"
                );

                if let Ok(mut pb) = playback.lock() {
                    // Get the current playback device name for logging
                    let current_device_name = pb.get_current_device();

                    tracing::debug!(
                        removed_device_id = %id,
                        current_device_name = %current_device_name,
                        "[DEVICE_MONITOR] Comparing removed device with current playback device"
                    );

                    // Use is_current_device for robust comparison
                    // This handles WinRT device IDs vs device names properly
                    if pb.is_current_device(&id) {
                        tracing::warn!(
                            device_id = %id,
                            "[DEVICE_MONITOR] Current playback device was removed - switching to default device"
                        );

                        // Check if a switch is already in progress
                        if pb.is_device_switching() {
                            tracing::debug!(
                                "[DEVICE_MONITOR] Device switch already in progress - skipping"
                            );
                        } else {
                            let backend = pb.get_current_backend();
                            match pb.switch_device_with_reason(
                                backend,
                                None,
                                DeviceSwitchReason::DeviceDisconnected,
                            ) {
                                Ok(()) => {
                                    tracing::info!(
                                        device_id = %id,
                                        "[DEVICE_MONITOR] Successfully switched to default device"
                                    );
                                }
                                Err(switch_err) => {
                                    tracing::error!(
                                        error = %switch_err,
                                        device_id = %id,
                                        "[DEVICE_MONITOR] Failed to switch to default device"
                                    );
                                }
                            }
                        }

                        // Emit to frontend
                        if let Err(e) = app_handle.emit(
                            "audio:device-removed",
                            serde_json::json!({
                                "id": id,
                                "switchingToDefault": true,
                            }),
                        ) {
                            tracing::warn!(
                                error = %e,
                                event = "audio:device-removed",
                                "[DEVICE_MONITOR] Failed to emit device event to frontend"
                            );
                        }
                    } else {
                        // Removed device was not the active playback device
                        tracing::debug!(
                            device_id = %id,
                            "[DEVICE_MONITOR] Removed device was not the active playback device - no action needed"
                        );

                        if let Err(e) = app_handle.emit(
                            "audio:device-removed",
                            serde_json::json!({
                                "id": id,
                                "switchingToDefault": false,
                            }),
                        ) {
                            tracing::warn!(
                                error = %e,
                                event = "audio:device-removed",
                                "[DEVICE_MONITOR] Failed to emit device event to frontend"
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        device_id = %id,
                        "[DEVICE_MONITOR] Failed to lock playback mutex for device removal"
                    );
                }
            }
            DeviceEvent::DefaultDeviceChanged { id, name } => {
                metrics.record_default_changed();

                tracing::info!(
                    device_id = %id,
                    device_name = %name,
                    "[DEVICE_MONITOR] Default device changed"
                );

                if let Ok(mut pb) = playback.lock() {
                    // Check if a switch is already in progress
                    if pb.is_device_switching() {
                        tracing::debug!(
                            "[DEVICE_MONITOR] Device switch already in progress - skipping default device switch"
                        );
                    } else {
                        // Switch to the new system default device
                        // This properly detects if we need to switch, unlike check_and_update_sample_rate
                        // which only checks sample rate on the *current* device
                        if let Err(e) = pb.switch_to_system_default() {
                            tracing::warn!(
                                error = %e,
                                device_id = %id,
                                device_name = %name,
                                "[DEVICE_MONITOR] Failed to switch to new default device"
                            );
                        } else {
                            tracing::info!(
                                device_name = %name,
                                native_device_id = %id,
                                "[DEVICE_MONITOR] Successfully switched to new default device"
                            );

                            // Store the native device ID for reliable device removal detection
                            // This allows us to precisely identify device removal events instead of
                            // relying on substring matching which can produce false positives
                            pb.set_native_device_id(Some(id.clone()));
                            tracing::debug!(
                                native_device_id = %id,
                                "[DEVICE_MONITOR] Stored native device ID for removal tracking"
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        device_id = %id,
                        device_name = %name,
                        "[DEVICE_MONITOR] Failed to lock playback mutex for default device change"
                    );
                }

                // Emit to frontend (no blocking operations)
                if let Err(e) = app_handle.emit(
                    "audio:default-device-changed",
                    serde_json::json!({
                        "id": id,
                        "name": name,
                    }),
                ) {
                    tracing::warn!(
                        error = %e,
                        event = "audio:default-device-changed",
                        "[DEVICE_MONITOR] Failed to emit device event to frontend"
                    );
                }
            }
            DeviceEvent::DevicePropertyChanged {
                ref id,
                ref property,
            } => {
                metrics.record_property_changed();

                tracing::debug!(
                    device_id = %id,
                    property = %property,
                    "[DEVICE_MONITOR] Device property changed"
                );

                // Property changes (like sample rate) will be caught by the
                // periodic sample rate check in event_emission_loop
            }
        }

        // Store this event as the last event (after processing to prevent skipping on errors)
        *last_event = Some(LastDeviceEvent {
            event_type,
            device_id,
            timestamp: Instant::now(),
        });
    }

    /// Async device monitoring task
    ///
    /// Watches for device changes (hotplug events) and handles them appropriately.
    /// This provides real-time notifications on platforms that support it:
    /// - macOS: CoreAudio property listeners (~1ms latency)
    /// - Linux: PipeWire registry events (~0ms latency)
    /// - Windows: WinRT DeviceWatcher (~0ms latency)
    /// - Fallback: CPAL polling (2s interval)
    ///
    /// Uses a bounded channel (capacity=8) to ensure proper ordering and backpressure.
    /// Events are processed sequentially by a dedicated task to prevent race conditions.
    ///
    /// # Cancellation
    /// The task can be gracefully cancelled via the `cancel_token`.
    async fn device_monitoring_task(
        playback: Arc<Mutex<DesktopPlayback>>,
        app_handle: AppHandle,
        metrics: Arc<DeviceMetrics>,
        cancel_token: CancellationToken,
    ) {
        let monitor = create_async_device_monitor();

        tracing::info!(
            platform = monitor.platform_name(),
            "[DEVICE_MONITOR] Starting async device monitoring"
        );

        // Create bounded channel for device events (capacity=8 for backpressure)
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<DeviceEvent>(8);

        // Spawn event processing task with deduplication tracker
        let playback_clone = playback.clone();
        let app_handle_clone = app_handle.clone();
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            let mut last_event: Option<LastDeviceEvent> = None;
            while let Some(event) = event_rx.recv().await {
                Self::handle_device_event(
                    event,
                    &playback_clone,
                    &app_handle_clone,
                    &metrics_clone,
                    &mut last_event,
                )
                .await;
            }
            tracing::info!("[DEVICE_MONITOR] Event processing task terminated");
        });

        // Clone sender for the polling fallback (callback moves the original below)
        let event_tx_poll = event_tx.clone();

        // Create callback that sends events to the channel
        let callback = Box::new(move |event: DeviceEvent| {
            // Send event to channel for ordered processing
            // Use try_send for non-blocking behavior in callback
            if let Err(e) = event_tx.try_send(event) {
                tracing::warn!(
                    error = %e,
                    "[DEVICE_MONITOR] Failed to send device event to processing channel (channel full or closed)"
                );
            }
        });

        match monitor.watch_for_changes(callback).await {
            Ok(_handle) => {
                tracing::info!(
                    platform = monitor.platform_name(),
                    "[DEVICE_MONITOR] Device monitoring active"
                );

                // Log metrics every 5 minutes while monitoring
                let mut metrics_interval = tokio::time::interval(Duration::from_secs(300));
                metrics_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                // Wait for cancellation signal or periodic metrics logging
                // The handle will be dropped when this task exits, triggering cleanup
                loop {
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            tracing::info!(
                                "[DEVICE_MONITOR] Cancellation signal received - stopping device monitoring"
                            );
                            // Log final metrics before exiting
                            metrics.log_summary();
                            break;
                        }
                        _ = metrics_interval.tick() => {
                            // Log periodic metrics summary
                            metrics.log_summary();
                        }
                    }
                }
            }
            Err(e) => {
                // Native watch_for_changes unavailable (e.g. WinRT DeviceWatcher on Windows).
                // Fall back to polling enumerate_devices() every 2 seconds so that OS-level
                // default device changes (e.g. switching output in Sound Settings) are detected.
                tracing::warn!(
                    error = %e,
                    platform = monitor.platform_name(),
                    "[DEVICE_MONITOR] Native watch unavailable — starting 2-second polling fallback"
                );

                let mut previous: Vec<soul_audio_desktop::AsyncDeviceInfo> = Vec::new();
                let mut poll_interval = tokio::time::interval(Duration::from_secs(2));
                poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                // Capture initial state so we don't fire spurious events on first tick
                if let Ok(devices) = monitor.enumerate_devices().await {
                    previous = devices;
                }

                loop {
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            tracing::info!(
                                "[DEVICE_MONITOR] Cancellation signal received — stopping polling fallback"
                            );
                            metrics.log_summary();
                            break;
                        }
                        _ = poll_interval.tick() => {
                            match monitor.enumerate_devices().await {
                                Ok(current) => {
                                    for event in soul_audio_desktop::detect_device_changes(&previous, &current) {
                                        if let Err(send_err) = event_tx_poll.try_send(event) {
                                            tracing::warn!(
                                                error = %send_err,
                                                "[DEVICE_MONITOR] Polling: event channel full, dropping event"
                                            );
                                        }
                                    }
                                    previous = current;
                                }
                                Err(poll_err) => {
                                    tracing::debug!(
                                        error = %poll_err,
                                        "[DEVICE_MONITOR] Polling: enumerate_devices failed, will retry"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Record a play event to the database
    ///
    /// Spawns a non-blocking async task to record the play.
    fn record_play_event(
        app_handle: &AppHandle,
        track_id: String,
        duration_secs: f64,
        completed: bool,
    ) {
        let app_handle_clone = app_handle.clone();

        let record_handle = tauri::async_runtime::spawn(async move {
            let app_state = app_handle_clone.state::<AppState>();
            let track_id_obj = soul_core::types::TrackId::new(track_id.clone());
            let user_id = soul_core::types::UserId::new(app_state.user_id.clone());

            tracing::debug!(
                track_id = %track_id,
                user_id = %app_state.user_id,
                duration_secs = duration_secs,
                completed = completed,
                "Recording track play"
            );

            if let Err(e) = soul_storage::tracks::record_play(
                &app_state.pool,
                user_id,
                track_id_obj,
                Some(duration_secs),
                completed,
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    track_id = %track_id,
                    "Failed to record play"
                );
            }
        });

        // Log errors from play recording
        tauri::async_runtime::spawn(async move {
            if let Err(e) = record_handle.await {
                tracing::error!("[PLAYBACK] Play recording task panicked: {:?}", e);
            }
        });
    }

    /// Handle batch loading request (forward pagination or jump)
    ///
    /// Queries the database based on the lazy context and appends tracks to the queue.
    async fn handle_batch_request(
        _playback: Arc<Mutex<DesktopPlayback>>,
        _app_handle: AppHandle,
        offset: usize,
        limit: usize,
        _is_jump: bool,
    ) {
        // Lazy loading was removed in Phase 4
        // This handler is deprecated
        tracing::warn!(
            "Batch load handler called but lazy loading was removed. offset={}, limit={}",
            offset,
            limit
        );
    }

    /// Play a track from local file
    ///
    /// # Arguments
    /// * `track` - Track metadata including file path
    pub fn play_track(&self, track: QueueTrack) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "play_track command".to_string(),
            })?;

        // Clear queue and add this track
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| AudioError::CommandFailed {
                command: "ClearQueue".to_string(),
                reason: e.to_string(),
            })?;

        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .map_err(|e| AudioError::CommandFailed {
                command: "AddToQueue".to_string(),
                reason: e.to_string(),
            })?;

        // Start playback
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| AudioError::CommandFailed {
                command: "Play".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Play
    pub fn play(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "play command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| AudioError::CommandFailed {
                command: "Play".to_string(),
                reason: e.to_string(),
            })
    }

    /// Pause
    pub fn pause(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "pause command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Pause)
            .map_err(|e| AudioError::CommandFailed {
                command: "Pause".to_string(),
                reason: e.to_string(),
            })
    }

    /// Stop
    pub fn stop(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "stop command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Stop)
            .map_err(|e| AudioError::CommandFailed {
                command: "Stop".to_string(),
                reason: e.to_string(),
            })
    }

    /// Next track
    pub fn next(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "next command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Next)
            .map_err(|e| AudioError::CommandFailed {
                command: "Next".to_string(),
                reason: e.to_string(),
            })
    }

    /// Previous track
    pub fn previous(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "previous command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Previous)
            .map_err(|e| AudioError::CommandFailed {
                command: "Previous".to_string(),
                reason: e.to_string(),
            })
    }

    /// Seek to position (in seconds)
    pub fn seek(&self, position: f64) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "seek command".to_string(),
            })?;

        playback
            .send_command(PlaybackCommand::Seek(position))
            .map_err(|e| AudioError::CommandFailed {
                command: "Seek".to_string(),
                reason: e.to_string(),
            })
    }

    /// Set volume (0-100)
    pub fn set_volume(&self, volume: u8) -> Result<(), AudioError> {
        let volume = volume.clamp(0, 100);
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_volume command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::SetVolume(volume))
            .map_err(|e| AudioError::CommandFailed {
                command: "SetVolume".to_string(),
                reason: e.to_string(),
            })
    }

    /// Mute
    pub fn mute(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "mute command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Mute)
            .map_err(|e| AudioError::CommandFailed {
                command: "Mute".to_string(),
                reason: e.to_string(),
            })
    }

    /// Unmute
    pub fn unmute(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "unmute command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::Unmute)
            .map_err(|e| AudioError::CommandFailed {
                command: "Unmute".to_string(),
                reason: e.to_string(),
            })
    }

    /// Set shuffle mode
    pub fn set_shuffle(&self, mode: ShuffleMode) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_shuffle command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::SetShuffle(mode))
            .map_err(|e| AudioError::CommandFailed {
                command: "SetShuffle".to_string(),
                reason: e.to_string(),
            })
    }

    /// Cycle shuffle mode (Off → Random → Smart → Off)
    ///
    /// Returns the new shuffle mode as a string
    pub fn cycle_shuffle(&self) -> Result<String, AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "cycle_shuffle command".to_string(),
            })?;

        // Cycle shuffle and get new mode synchronously
        let new_mode = playback
            .get_playback_manager()
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "cycle_shuffle inner manager".to_string(),
            })
            .map(|mut mgr| mgr.cycle_shuffle())?;

        // Emit queue updated event
        playback.emit_queue_updated();

        tracing::debug!(mode = new_mode.as_str(), "Cycled shuffle");
        Ok(new_mode.as_str().to_string())
    }

    /// Get current shuffle mode
    pub fn get_shuffle(&self) -> ShuffleMode {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting shuffle mode - audio thread may have crashed");
            return ShuffleMode::Off;
        };
        playback.get_shuffle_mode()
    }

    /// Get current repeat mode
    pub fn get_repeat(&self) -> RepeatMode {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting repeat mode - audio thread may have crashed"
            );
            return RepeatMode::Off;
        };
        playback.get_repeat_mode()
    }

    /// Set repeat mode
    pub fn set_repeat(&self, mode: RepeatMode) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_repeat command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::SetRepeat(mode))
            .map_err(|e| AudioError::CommandFailed {
                command: "SetRepeat".to_string(),
                reason: e.to_string(),
            })
    }

    /// Cycle through repeat modes: Off → All → One → Off
    pub fn cycle_repeat(&self) -> Result<String, AudioError> {
        let current = self.get_repeat();
        let next = match current {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };

        self.set_repeat(next)?;

        Ok(match next {
            RepeatMode::Off => "off".to_string(),
            RepeatMode::All => "all".to_string(),
            RepeatMode::One => "one".to_string(),
        })
    }

    /// Get queue
    pub fn get_queue(&self) -> Vec<QueueTrack> {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting queue - audio thread may have crashed"
            );
            return Vec::new();
        };
        playback.get_queue()
    }

    /// Check if there is a next track
    pub fn has_next(&self) -> bool {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while checking has_next - audio thread may have crashed"
            );
            return false;
        };
        playback.has_next()
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while checking has_previous - audio thread may have crashed");
            return false;
        };
        playback.has_previous()
    }

    /// Get current playback state
    pub fn get_state(&self) -> soul_playback::PlaybackState {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting playback state - audio thread may have crashed");
            return soul_playback::PlaybackState::Stopped;
        };
        playback.get_state()
    }

    /// Get current track information
    pub fn get_current_track(&self) -> Option<QueueTrack> {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting current track - audio thread may have crashed");
            return None;
        };
        playback.get_current_track()
    }

    /// Get current queue index (0 if playing, -1 if stopped)
    pub fn get_queue_index(&self) -> i32 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting queue index - audio thread may have crashed"
            );
            return -1;
        };
        playback.get_queue_index()
    }

    /// Get current playback position in seconds
    pub fn get_position(&self) -> f64 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting position - audio thread may have crashed"
            );
            return 0.0;
        };
        playback.get_position().as_secs_f64()
    }

    /// Get current volume (0.0 to 1.0)
    pub fn get_volume(&self) -> f64 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting volume - audio thread may have crashed"
            );
            return 0.0;
        };
        playback.get_volume() as f64 / 100.0
    }

    /// Add track to queue (legacy - maps to add_to_queue_end)
    pub fn add_to_queue(&self, track: QueueTrack) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "add_to_queue command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .map_err(|e| AudioError::CommandFailed {
                command: "AddToQueue".to_string(),
                reason: e.to_string(),
            })
    }

    /// Add track to Play Next queue (plays after current track)
    pub fn add_play_next(&self, track: QueueTrack) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "add_play_next command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::AddPlayNext(track))
            .map_err(|e| AudioError::CommandFailed {
                command: "AddPlayNext".to_string(),
                reason: e.to_string(),
            })
    }

    /// Add track to end of Add to Queue (plays after source exhausts)
    pub fn add_to_queue_end(&self, track: QueueTrack) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "add_to_queue_end command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::AddToQueueEnd(track))
            .map_err(|e| AudioError::CommandFailed {
                command: "AddToQueueEnd".to_string(),
                reason: e.to_string(),
            })
    }

    /// Remove track from queue by index
    pub fn remove_from_queue(&self, index: usize) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "remove_from_queue command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::RemoveFromQueue(index))
            .map_err(|e| AudioError::CommandFailed {
                command: "RemoveFromQueue".to_string(),
                reason: e.to_string(),
            })
    }

    /// Clear entire queue (all three tiers)
    pub fn clear_queue(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "clear_queue command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| AudioError::CommandFailed {
                command: "ClearQueue".to_string(),
                reason: e.to_string(),
            })
    }

    /// Clear Play Next queue only
    pub fn clear_play_next(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "clear_play_next command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::ClearPlayNext)
            .map_err(|e| AudioError::CommandFailed {
                command: "ClearPlayNext".to_string(),
                reason: e.to_string(),
            })
    }

    /// Clear Add to Queue only
    pub fn clear_add_to_queue(&self) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "clear_add_to_queue command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::ClearAddToQueue)
            .map_err(|e| AudioError::CommandFailed {
                command: "ClearAddToQueue".to_string(),
                reason: e.to_string(),
            })
    }

    /// Skip to track at queue index
    pub fn skip_to_queue_index(&self, index: usize) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "skip_to_queue_index command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::SkipToQueueIndex(index))
            .map_err(|e| AudioError::CommandFailed {
                command: "SkipToQueueIndex".to_string(),
                reason: e.to_string(),
            })
    }

    /// Load playlist/album as source queue (replaces playback context)
    pub fn load_playlist(
        &self,
        tracks: Vec<QueueTrack>,
        start_index: usize,
    ) -> Result<(), AudioError> {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "load_playlist command".to_string(),
            })?;
        playback
            .send_command(PlaybackCommand::LoadPlaylist {
                tracks,
                start_index,
            })
            .map_err(|e| AudioError::CommandFailed {
                command: "LoadPlaylist".to_string(),
                reason: e.to_string(),
            })
    }

    /// Set lazy context for on-demand track loading
    pub fn set_lazy_context(
        &self,
        context: QueueContext,
        _shuffle_seed: Option<u64>,
    ) -> Result<(), AudioError> {
        // Lazy loading removed in Phase 4 - all tracks loaded eagerly
        tracing::debug!("[set_playback_context] Context set: {:?}", context);
        Ok(())
    }

    /// Skip to track at index (alias for skip_to_queue_index)
    pub fn skip_to_index(&self, index: usize) -> Result<(), AudioError> {
        self.skip_to_queue_index(index)
    }

    /// Switch audio output device
    ///
    /// # Arguments
    /// * `backend` - Audio backend to use
    /// * `device_name` - Device name to switch to (None for default device)
    ///
    /// # Returns
    /// * `Ok(())` - Device switched successfully
    /// * `Err(_)` - Failed to switch device
    pub fn switch_device(
        &self,
        backend: soul_audio_desktop::AudioBackend,
        device_name: Option<String>,
    ) -> Result<(), AudioError> {
        // Record switch attempt
        self.device_metrics.record_switch_start();

        let start = Instant::now();

        tracing::debug!("Acquiring lock for device switch");
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "switch_device command".to_string(),
            })?;
        tracing::debug!("Lock acquired, switching device");
        let result = playback.switch_device(backend, device_name);

        // Record metrics based on result
        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => {
                self.device_metrics.record_switch_success(duration_ms);
                tracing::debug!(
                    duration_ms = duration_ms,
                    "Device switch completed successfully"
                );
            }
            Err(e) => {
                self.device_metrics.record_switch_failure();
                tracing::debug!(
                    duration_ms = duration_ms,
                    error = %e,
                    "Device switch failed"
                );
            }
        }

        // Explicitly drop the guard to release the lock
        drop(playback);
        result
    }

    /// Get current audio backend
    pub fn get_current_backend(&self) -> soul_audio_desktop::AudioBackend {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting audio backend - audio thread may have crashed");
            return soul_audio_desktop::AudioBackend::Default;
        };
        playback.get_current_backend()
    }

    /// Get current device name
    pub fn get_current_device(&self) -> String {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting device name - audio thread may have crashed"
            );
            return "Unknown Device".to_string();
        };
        playback.get_current_device()
    }

    /// Get current sample rate
    pub fn get_current_sample_rate(&self) -> u32 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while getting sample rate - audio thread may have crashed"
            );
            return 44100;
        };
        playback.get_current_sample_rate()
    }

    /// Manually trigger a sample rate check and update
    ///
    /// This is useful when the user knows they've changed device settings
    /// and wants to immediately update without waiting for the next poll.
    ///
    /// # Returns
    /// * `Ok(true)` - Sample rate changed and stream was recreated
    /// * `Ok(false)` - Sample rate unchanged
    /// * `Err(_)` - Failed to check or update
    pub fn refresh_sample_rate(&self) -> Result<bool, AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "refresh_sample_rate command".to_string(),
            })?;
        playback.check_and_update_sample_rate()
    }

    // ===== DSP Effect Chain =====

    /// Get effect slots state
    #[cfg(feature = "effects")]
    pub fn get_effect_slots(
        &self,
    ) -> Result<[Option<crate::dsp_commands::EffectSlotState>; 4], AudioError> {
        let slots = self
            .effect_slots
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "get_effect_slots".to_string(),
            })?;
        Ok(slots.clone())
    }

    /// Set effect in a slot and rebuild the effect chain
    #[cfg(feature = "effects")]
    pub fn set_effect_slot(
        &self,
        slot_index: usize,
        effect: Option<crate::dsp_commands::EffectSlotState>,
    ) -> Result<(), AudioError> {
        if slot_index >= 4 {
            return Err(AudioError::DeviceError(
                "Slot index must be 0-3".to_string(),
            ));
        }

        // Update slot
        {
            let mut slots = self
                .effect_slots
                .lock()
                .map_err(|_| AudioError::MutexPoisoned {
                    context: "set_effect_slot".to_string(),
                })?;
            slots[slot_index] = effect;
        }

        // Rebuild effect chain
        self.rebuild_effect_chain()
    }

    /// Update effect parameters in-place WITHOUT rebuilding the chain
    ///
    /// This preserves filter states and prevents audio artifacts (sizzle/pops)
    /// that occur when effects are recreated during parameter drags.
    #[cfg(feature = "effects")]
    pub fn update_effect_parameters_in_place(
        &self,
        slot_index: usize,
        effect: &crate::dsp_commands::EffectType,
    ) -> Result<bool, AudioError> {
        use crate::dsp_commands::EffectType;
        use soul_audio::effects::{
            Compressor, Crossfeed, CrossfeedPreset, GraphicEq, Limiter, ParametricEq,
            StereoEnhancer,
        };

        if slot_index >= 4 {
            return Err(AudioError::DeviceError(
                "Slot index must be 0-3".to_string(),
            ));
        }

        // Try to update in-place
        let updated = self.with_effect_chain(|chain| {
            match effect {
                EffectType::Eq { bands } => {
                    if let Some(eq) = chain.get_effect_as_mut::<ParametricEq>(slot_index) {
                        eq.set_bands(bands.iter().map(|b| b.clone().into()).collect());
                        true
                    } else {
                        false
                    }
                }
                EffectType::GraphicEq { settings } => {
                    if let Some(geq) = chain.get_effect_as_mut::<GraphicEq>(slot_index) {
                        for (i, &gain) in settings.gains.iter().enumerate() {
                            geq.set_band_gain(i, gain);
                        }
                        true
                    } else {
                        false
                    }
                }
                EffectType::Limiter { settings } => {
                    if let Some(lim) = chain.get_effect_as_mut::<Limiter>(slot_index) {
                        lim.set_threshold(settings.threshold_db);
                        lim.set_release(settings.release_ms);
                        true
                    } else {
                        false
                    }
                }
                EffectType::Compressor { settings } => {
                    if let Some(comp) = chain.get_effect_as_mut::<Compressor>(slot_index) {
                        // Use set_settings to update all parameters including knee
                        comp.set_settings(settings.clone().into());
                        true
                    } else {
                        false
                    }
                }
                EffectType::Stereo { settings } => {
                    if let Some(stereo) = chain.get_effect_as_mut::<StereoEnhancer>(slot_index) {
                        stereo.set_width(settings.width);
                        stereo.set_mid_gain_db(settings.mid_gain_db);
                        stereo.set_side_gain_db(settings.side_gain_db);
                        stereo.set_balance(settings.balance);
                        true
                    } else {
                        false
                    }
                }
                EffectType::Crossfeed { settings } => {
                    if let Some(cf) = chain.get_effect_as_mut::<Crossfeed>(slot_index) {
                        let preset = match settings.preset.as_str() {
                            "natural" => CrossfeedPreset::Natural,
                            "relaxed" => CrossfeedPreset::Relaxed,
                            "meier" => CrossfeedPreset::Meier,
                            _ => CrossfeedPreset::Custom,
                        };

                        if preset == CrossfeedPreset::Custom {
                            cf.set_level_db(settings.level_db);
                            cf.set_cutoff_hz(settings.cutoff_hz);
                        } else {
                            cf.set_preset(preset);
                        }
                        true
                    } else {
                        false
                    }
                }
                EffectType::Convolution { .. } => {
                    // Convolution can't be updated in-place (needs IR reload)
                    false
                }
            }
        })?;

        // Also update the stored slot state
        if updated {
            let mut slots = self
                .effect_slots
                .lock()
                .map_err(|_| AudioError::MutexPoisoned {
                    context: "update_effect_parameters_in_place".to_string(),
                })?;
            if let Some(ref mut slot_state) = slots[slot_index] {
                slot_state.effect = effect.clone();
            }
        }

        Ok(updated)
    }

    /// Rebuild the entire effect chain from current slot state
    #[cfg(feature = "effects")]
    fn rebuild_effect_chain(&self) -> Result<(), AudioError> {
        use crate::dsp_commands::EffectType;
        use soul_audio::effects::{
            AudioEffect, Compressor, ConvolutionEngine, Crossfeed, CrossfeedPreset,
            CrossfeedSettings, GraphicEq, GraphicEqBands, Limiter, ParametricEq, StereoEnhancer,
            StereoSettings,
        };

        let slots = self
            .effect_slots
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "rebuild_effect_chain".to_string(),
            })?;

        self.with_effect_chain(|chain| {
            // Clear existing effects
            chain.clear();

            // Add effects from slots
            for slot in slots.iter() {
                if let Some(slot_state) = slot {
                    let effect: Box<dyn soul_audio::effects::AudioEffect> = match &slot_state.effect
                    {
                        EffectType::Eq { bands } => {
                            let mut eq = ParametricEq::new();
                            eq.set_bands(bands.iter().map(|b| b.clone().into()).collect());
                            eq.set_enabled(slot_state.enabled);
                            Box::new(eq)
                        }
                        EffectType::Compressor { settings } => {
                            let mut comp = Compressor::with_settings(settings.clone().into());
                            comp.set_enabled(slot_state.enabled);
                            Box::new(comp)
                        }
                        EffectType::Limiter { settings } => {
                            let mut lim = Limiter::with_settings(settings.clone().into());
                            lim.set_enabled(slot_state.enabled);
                            Box::new(lim)
                        }
                        EffectType::Crossfeed { settings } => {
                            let preset = match settings.preset.as_str() {
                                "natural" => CrossfeedPreset::Natural,
                                "relaxed" => CrossfeedPreset::Relaxed,
                                "meier" => CrossfeedPreset::Meier,
                                _ => CrossfeedPreset::Custom,
                            };

                            let crossfeed_settings = if preset == CrossfeedPreset::Custom {
                                CrossfeedSettings::custom(settings.level_db, settings.cutoff_hz)
                            } else {
                                CrossfeedSettings::from_preset(preset)
                            };

                            let mut crossfeed = Crossfeed::with_settings(crossfeed_settings);
                            crossfeed.set_enabled(slot_state.enabled);
                            Box::new(crossfeed)
                        }
                        EffectType::Stereo { settings } => {
                            let stereo_settings = StereoSettings {
                                width: settings.width,
                                mid_gain_db: settings.mid_gain_db,
                                side_gain_db: settings.side_gain_db,
                                balance: settings.balance,
                            };

                            let mut stereo = StereoEnhancer::with_settings(stereo_settings);
                            stereo.set_enabled(slot_state.enabled);
                            Box::new(stereo)
                        }
                        EffectType::GraphicEq { settings } => {
                            let mut graphic_eq = if settings.band_count == 31 {
                                GraphicEq::new(GraphicEqBands::ThirtyOne)
                            } else {
                                GraphicEq::new_10_band()
                            };

                            // Apply gains if we have the right number
                            if settings.band_count == 10 && settings.gains.len() == 10 {
                                if let Ok(gains) = settings.gains.clone().try_into() {
                                    graphic_eq.set_gains_10(gains);
                                }
                            } else {
                                // For 31-band or custom, set each band individually
                                for (i, &gain) in settings.gains.iter().enumerate() {
                                    graphic_eq.set_band_gain(i, gain);
                                }
                            }

                            graphic_eq.set_enabled(slot_state.enabled);
                            Box::new(graphic_eq)
                        }
                        EffectType::Convolution { settings } => {
                            let mut conv = ConvolutionEngine::new();

                            // Load IR from file path if provided
                            if !settings.ir_file_path.is_empty() {
                                match conv.load_from_wav(&settings.ir_file_path) {
                                    Ok(()) => {
                                        conv.set_dry_wet_mix(settings.wet_dry_mix);
                                        // Note: pre_delay_ms and decay are UI-only for now
                                        // The ConvolutionEngine applies full IR as-is
                                        tracing::debug!(
                                            ir_file_path = %settings.ir_file_path,
                                            "Loaded IR"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            ir_file_path = %settings.ir_file_path,
                                            error = %e,
                                            "Failed to load IR file"
                                        );
                                        // Keep the engine but it won't process anything
                                    }
                                }
                            }

                            conv.set_enabled(slot_state.enabled);
                            Box::new(conv)
                        }
                    };
                    chain.add_effect(effect);
                }
            }
        })
    }

    /// Access the effect chain for configuration
    #[cfg(feature = "effects")]
    pub fn with_effect_chain<F, R>(&self, f: F) -> Result<R, AudioError>
    where
        F: FnOnce(&mut soul_audio::effects::EffectChain) -> R,
    {
        let playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "with_effect_chain".to_string(),
            })?;
        Ok(playback.with_effect_chain(f))
    }

    // ===== Volume Leveling =====

    /// Set volume leveling mode (ReplayGain track/album, EBU R128, etc.)
    pub fn set_volume_leveling_mode(&self, mode: soul_playback::NormalizationMode) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting volume leveling mode - audio thread may have crashed");
            return;
        };
        playback.set_volume_leveling_mode(mode);
    }

    /// Get current volume leveling mode
    pub fn get_volume_leveling_mode(&self) -> soul_playback::NormalizationMode {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting volume leveling mode - audio thread may have crashed");
            return soul_playback::NormalizationMode::Disabled;
        };
        playback.get_volume_leveling_mode()
    }

    /// Set track gain for current track (called when loading track)
    pub fn set_track_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while setting track gain - audio thread may have crashed"
            );
            return;
        };
        playback.set_track_gain(gain_db, peak_dbfs);
    }

    /// Set album gain for current track (called when loading track)
    pub fn set_album_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!(
                "Playback mutex poisoned while setting album gain - audio thread may have crashed"
            );
            return;
        };
        playback.set_album_gain(gain_db, peak_dbfs);
    }

    /// Clear gain values (for new track without loudness data)
    pub fn clear_loudness_gains(&self) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while clearing loudness gains - audio thread may have crashed");
            return;
        };
        playback.clear_loudness_gains();
    }

    /// Set pre-amp gain for volume leveling
    pub fn set_loudness_preamp(&self, preamp_db: f64) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting loudness preamp - audio thread may have crashed");
            return;
        };
        playback.set_loudness_preamp(preamp_db);
    }

    /// Set whether to prevent clipping during volume leveling
    pub fn set_prevent_clipping(&self, prevent: bool) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting prevent clipping - audio thread may have crashed");
            return;
        };
        playback.set_prevent_clipping(prevent);
    }

    // ===== Exclusive Mode / Bit-Perfect Output =====

    /// Get current latency information
    pub fn get_latency_info(&self) -> LatencyInfo {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting latency info - audio thread may have crashed");
            return LatencyInfo::default();
        };
        playback.get_latency_info()
    }

    /// Enable exclusive mode with configuration
    ///
    /// This switches to WASAPI exclusive mode (Windows) or maintains
    /// ASIO/JACK if configured, providing:
    /// - Bit-perfect output (no OS mixer)
    /// - Lower latency
    /// - Direct sample format control
    pub fn set_exclusive_mode(&self, config: ExclusiveConfig) -> Result<LatencyInfo, AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_exclusive_mode command".to_string(),
            })?;
        playback.set_exclusive_mode(config)
    }

    /// Disable exclusive mode (return to shared mode)
    pub fn disable_exclusive_mode(&self) -> Result<(), AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "disable_exclusive_mode command".to_string(),
            })?;
        playback.disable_exclusive_mode()
    }

    /// Check if currently in exclusive mode
    pub fn is_exclusive_mode(&self) -> bool {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while checking exclusive mode - audio thread may have crashed");
            return false;
        };
        playback.is_exclusive_mode()
    }

    // ===== Crossfade Settings =====

    /// Set crossfade enabled/disabled
    pub fn set_crossfade_enabled(&self, enabled: bool) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting crossfade enabled - audio thread may have crashed");
            return;
        };
        playback.set_crossfade_enabled(enabled);
    }

    /// Get current crossfade enabled state
    pub fn is_crossfade_enabled(&self) -> bool {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while checking crossfade enabled - audio thread may have crashed");
            return false;
        };
        playback.is_crossfade_enabled()
    }

    /// Set crossfade duration in milliseconds
    pub fn set_crossfade_duration(&self, duration_ms: u32) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting crossfade duration - audio thread may have crashed");
            return;
        };
        playback.set_crossfade_duration(duration_ms);
    }

    /// Get crossfade duration in milliseconds
    pub fn get_crossfade_duration(&self) -> u32 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting crossfade duration - audio thread may have crashed");
            return 5000;
        };
        playback.get_crossfade_duration()
    }

    /// Set crossfade curve type
    pub fn set_crossfade_curve(&self, curve: soul_playback::FadeCurve) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting crossfade curve - audio thread may have crashed");
            return;
        };
        playback.set_crossfade_curve(curve);
    }

    /// Get crossfade curve type
    pub fn get_crossfade_curve(&self) -> soul_playback::FadeCurve {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting crossfade curve - audio thread may have crashed");
            return soul_playback::FadeCurve::Linear;
        };
        playback.get_crossfade_curve()
    }

    // ===========================================================================
    // Resampling Settings
    // ===========================================================================

    /// Set resampling quality preset
    ///
    /// Quality presets control the filter parameters used during sample rate conversion:
    /// - "fast": 64-tap filter, 0.90 cutoff - low CPU usage
    /// - "balanced": 128-tap filter, 0.95 cutoff - good quality
    /// - "high": 256-tap filter, 0.99 cutoff - excellent quality (default)
    /// - "maximum": 512-tap filter, 0.995 cutoff - audiophile quality
    ///
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_quality(&self, quality: &str) -> Result<(), AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_resampling_quality command".to_string(),
            })?;
        playback
            .set_resampling_quality(quality)
            .map_err(|e| AudioError::DeviceError(e))
    }

    /// Get current resampling quality preset
    pub fn get_resampling_quality(&self) -> String {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting resampling quality - audio thread may have crashed");
            return "high".to_string();
        };
        playback.get_resampling_quality()
    }

    /// Set resampling target sample rate
    ///
    /// - rate=0: Auto mode - match device native sample rate
    /// - rate>0: Force specific output sample rate (e.g., 96000)
    ///
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_target_rate(&self, rate: u32) -> Result<(), AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_resampling_target_rate command".to_string(),
            })?;
        playback
            .set_resampling_target_rate(rate)
            .map_err(|e| AudioError::DeviceError(e))
    }

    /// Get current resampling target sample rate
    ///
    /// Returns 0 for auto mode, or the specific rate in Hz.
    pub fn get_resampling_target_rate(&self) -> u32 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting resampling target rate - audio thread may have crashed");
            return 0;
        };
        playback.get_resampling_target_rate()
    }

    /// Set resampling backend
    ///
    /// Backends:
    /// - "auto": Use best available (r8brain if compiled in, else rubato)
    /// - "rubato": Use Rubato library (always available)
    /// - "r8brain": Use r8brain library (requires feature flag)
    ///
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_backend(&self, backend: &str) -> Result<(), AudioError> {
        let mut playback = self
            .playback
            .lock()
            .map_err(|_| AudioError::MutexPoisoned {
                context: "set_resampling_backend command".to_string(),
            })?;
        playback
            .set_resampling_backend(backend)
            .map_err(|e| AudioError::DeviceError(e))
    }

    /// Get current resampling backend
    pub fn get_resampling_backend(&self) -> String {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting resampling backend - audio thread may have crashed");
            return "auto".to_string();
        };
        playback.get_resampling_backend()
    }

    // ===== Headroom Management =====

    /// Set headroom management mode
    pub fn set_headroom_mode(&self, mode: soul_playback::HeadroomMode) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting headroom mode - audio thread may have crashed");
            return;
        };
        playback.set_headroom_mode(mode);
    }

    /// Get current headroom mode
    pub fn get_headroom_mode(&self) -> soul_playback::HeadroomMode {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting headroom mode - audio thread may have crashed");
            return soul_playback::HeadroomMode::Disabled;
        };
        playback.get_headroom_mode()
    }

    /// Set headroom enabled state
    pub fn set_headroom_enabled(&self, enabled: bool) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting headroom enabled - audio thread may have crashed");
            return;
        };
        playback.set_headroom_enabled(enabled);
    }

    /// Check if headroom management is enabled
    pub fn is_headroom_enabled(&self) -> bool {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while checking headroom enabled - audio thread may have crashed");
            return false;
        };
        playback.is_headroom_enabled()
    }

    /// Set EQ boost value for headroom calculation
    pub fn set_headroom_eq_boost_db(&self, boost_db: f64) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting headroom EQ boost - audio thread may have crashed");
            return;
        };
        playback.set_headroom_eq_boost_db(boost_db);
    }

    /// Set pre-amp value for headroom calculation
    pub fn set_headroom_preamp_db(&self, preamp_db: f64) {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while setting headroom preamp - audio thread may have crashed");
            return;
        };
        playback.set_headroom_preamp_db(preamp_db);
    }

    /// Get total potential gain from all sources
    pub fn get_headroom_total_gain_db(&self) -> f64 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting headroom total gain - audio thread may have crashed");
            return 0.0;
        };
        playback.get_headroom_total_gain_db()
    }

    /// Get current attenuation being applied
    pub fn get_headroom_attenuation_db(&self) -> f64 {
        let Ok(playback) = self.playback.lock() else {
            tracing::error!("Playback mutex poisoned while getting headroom attenuation - audio thread may have crashed");
            return 0.0;
        };
        playback.get_headroom_attenuation_db()
    }

    // ===== Device Monitoring Metrics =====

    /// Get device monitoring metrics snapshot
    ///
    /// Returns a snapshot of current device event and switch metrics for observability.
    /// All counters are thread-safe and can be queried at any time without blocking.
    pub fn get_device_metrics(&self) -> DeviceMetricsSnapshot {
        self.device_metrics.snapshot()
    }

    /// Stop device monitoring gracefully
    ///
    /// This sends a cancellation signal to the device monitoring task,
    /// allowing it to clean up resources and exit gracefully.
    pub fn stop_device_monitoring(&self) {
        tracing::debug!("[DEVICE_MONITOR] Sending cancellation signal to device monitoring task");
        self.device_monitor_cancel_token.cancel();
    }
}

impl Drop for PlaybackManager {
    fn drop(&mut self) {
        tracing::debug!("[PlaybackManager] Dropping PlaybackManager - stopping device monitoring");
        self.stop_device_monitoring();
    }
}

#[cfg(test)]
mod tests {
    // Tests removed - device_monitor functionality has been replaced with async event-based monitoring
    // See soul_audio_desktop::create_async_device_monitor() for the new implementation
}
