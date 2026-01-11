//! Filesystem watcher for library sources
//!
//! Watches library source directories for changes and triggers appropriate
//! scan operations. Uses debouncing to avoid processing the same file multiple times.
//!
//! # Platform Support
//!
//! - Windows: `ReadDirectoryChangesW`
//! - macOS: `FSEvents`
//! - Linux: `inotify`

use crate::{library_scanner::LibraryScanner, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use soul_core::types::LibrarySource;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Default debounce duration in milliseconds
const DEFAULT_DEBOUNCE_MS: u64 = 500;

/// Filesystem event that has been debounced and is ready for processing
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// A file was created or moved into the watched directory
    Created(PathBuf),
    /// A file was modified
    Modified(PathBuf),
    /// A file was deleted or moved out of the watched directory
    Removed(PathBuf),
    /// A file was renamed (old path, new path)
    Renamed(PathBuf, PathBuf),
}

/// Callback for watcher events
pub type EventCallback = Box<dyn Fn(WatcherEvent) + Send + Sync>;

/// Configuration for the library watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration for filesystem events
    pub debounce_duration: Duration,
    /// Whether to process events immediately or batch them
    pub batch_events: bool,
    /// Maximum batch size before forcing a scan
    pub max_batch_size: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(DEFAULT_DEBOUNCE_MS),
            batch_events: true,
            max_batch_size: 100,
        }
    }
}

/// Watches library source directories for filesystem changes
pub struct LibraryWatcher {
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    config: WatcherConfig,
    /// Active watchers by source ID
    watchers: Arc<RwLock<HashMap<i64, WatcherHandle>>>,
    /// Event sender for processing
    event_tx: mpsc::Sender<(i64, WatcherEvent)>,
    /// Event receiver for processing (held by the processing task)
    event_rx: Option<mpsc::Receiver<(i64, WatcherEvent)>>,
}

/// Handle to a single directory watcher
struct WatcherHandle {
    #[allow(dead_code)]
    source_id: i64,
    path: PathBuf,
    // The debouncer owns the watcher, so we need to keep it alive
    #[allow(dead_code)]
    debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl LibraryWatcher {
    /// Create a new library watcher
    pub fn new(pool: SqlitePool, user_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        Self {
            pool,
            user_id: user_id.into(),
            device_id: device_id.into(),
            config: WatcherConfig::default(),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    /// Set watcher configuration
    pub fn with_config(mut self, config: WatcherConfig) -> Self {
        self.config = config;
        self
    }

    /// Start watching all enabled library sources
    pub async fn start_watching(&self) -> Result<()> {
        let sources =
            soul_storage::library_sources::get_enabled(&self.pool, &self.user_id, &self.device_id)
                .await?;

        for source in sources {
            if let Err(e) = self.watch_source(&source).await {
                error!("Failed to watch source {}: {}", source.name, e);
            }
        }

        Ok(())
    }

    /// Start watching a specific library source
    pub async fn watch_source(&self, source: &LibrarySource) -> Result<()> {
        let source_path = Path::new(&source.path);

        // Verify path exists
        if !source_path.exists() {
            warn!("Cannot watch non-existent path: {}", source.path);
            return Ok(());
        }

        let source_id = source.id;
        let event_tx = self.event_tx.clone();

        // Create debouncer with callback
        let debouncer = new_debouncer(
            self.config.debounce_duration,
            None, // No tick rate
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events {
                            if let Some(watcher_event) = convert_event(&event.event) {
                                // Send event for processing
                                let _ = event_tx.blocking_send((source_id, watcher_event));
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            error!("Watcher error: {:?}", error);
                        }
                    }
                }
            },
        )
        .map_err(|e| crate::ImportError::Unknown(format!("Failed to create debouncer: {}", e)))?;

        // Watch the source path with the debouncer
        let mut debouncer = debouncer;
        debouncer
            .watch(source_path, RecursiveMode::Recursive)
            .map_err(|e| crate::ImportError::Unknown(format!("Failed to watch path: {}", e)))?;

        // Store the watcher handle
        let handle = WatcherHandle {
            source_id,
            path: source_path.to_path_buf(),
            debouncer,
        };

        let mut watchers = self.watchers.write().await;
        watchers.insert(source_id, handle);

        info!("Started watching: {} ({})", source.name, source.path);
        Ok(())
    }

    /// Stop watching a specific library source
    pub async fn unwatch_source(&self, source_id: i64) -> Result<()> {
        let mut watchers = self.watchers.write().await;

        if let Some(handle) = watchers.remove(&source_id) {
            info!("Stopped watching: {:?}", handle.path);
        }

        Ok(())
    }

    /// Stop watching all sources
    pub async fn stop_watching(&self) -> Result<()> {
        let mut watchers = self.watchers.write().await;
        watchers.clear();
        info!("Stopped all watchers");
        Ok(())
    }

    /// Take the event receiver for processing
    ///
    /// This can only be called once. The receiver is used to process
    /// filesystem events as they arrive.
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<(i64, WatcherEvent)>> {
        self.event_rx.take()
    }

    /// Get the number of active watchers
    pub async fn watcher_count(&self) -> usize {
        self.watchers.read().await.len()
    }

    /// Check if a specific source is being watched
    pub async fn is_watching(&self, source_id: i64) -> bool {
        self.watchers.read().await.contains_key(&source_id)
    }
}

/// Event processor that handles watcher events and updates the library
pub struct EventProcessor {
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    /// Pending events batched by source ID
    pending: HashMap<i64, Vec<WatcherEvent>>,
    /// Maximum batch size before forcing a scan
    max_batch_size: usize,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(pool: SqlitePool, user_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            pool,
            user_id: user_id.into(),
            device_id: device_id.into(),
            pending: HashMap::new(),
            max_batch_size: 100,
        }
    }

    /// Set the maximum batch size
    pub fn max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Process a single event
    pub async fn process_event(&mut self, source_id: i64, event: WatcherEvent) -> Result<()> {
        debug!("Processing event for source {}: {:?}", source_id, event);

        // Add to pending batch
        self.pending.entry(source_id).or_default().push(event);

        // Check if we should flush
        if let Some(events) = self.pending.get(&source_id) {
            if events.len() >= self.max_batch_size {
                self.flush_source(source_id).await?;
            }
        }

        Ok(())
    }

    /// Flush pending events for a source
    pub async fn flush_source(&mut self, source_id: i64) -> Result<()> {
        if let Some(events) = self.pending.remove(&source_id) {
            if events.is_empty() {
                return Ok(());
            }

            info!("Flushing {} events for source {}", events.len(), source_id);

            // Get the source
            let source = soul_storage::library_sources::get_by_id(&self.pool, source_id).await?;
            let Some(source) = source else {
                warn!("Source {} not found, skipping events", source_id);
                return Ok(());
            };

            // Process events
            for event in events {
                if let Err(e) = self.handle_event(&source, event).await {
                    error!("Failed to handle event: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Flush all pending events
    pub async fn flush_all(&mut self) -> Result<()> {
        let source_ids: Vec<i64> = self.pending.keys().copied().collect();

        for source_id in source_ids {
            self.flush_source(source_id).await?;
        }

        Ok(())
    }

    /// Handle a single event
    async fn handle_event(&self, source: &LibrarySource, event: WatcherEvent) -> Result<()> {
        let scanner = LibraryScanner::new(
            self.pool.clone(),
            self.user_id.clone(),
            self.device_id.clone(),
        );

        match event {
            WatcherEvent::Created(path) | WatcherEvent::Modified(path) => {
                debug!("Processing created/modified file: {:?}", path);

                // Check if it's an audio file
                if !is_audio_file(&path) {
                    return Ok(());
                }

                // Use the scanner to process this single file
                // The scanner already handles change detection and updates
                let stats = scanner.scan_source(source).await?;
                debug!(
                    "Scan complete: {} new, {} updated",
                    stats.new_files, stats.updated_files
                );
            }
            WatcherEvent::Removed(path) => {
                debug!("Processing removed file: {:?}", path);

                // Check if it's an audio file
                if !is_audio_file(&path) {
                    return Ok(());
                }

                // The next scan will detect the missing file and mark it unavailable
                // For now, we trigger a full scan which will handle it
                if source.sync_deletes {
                    let stats = scanner.scan_source(source).await?;
                    debug!("Scan complete: {} removed", stats.removed_files);
                }
            }
            WatcherEvent::Renamed(old_path, new_path) => {
                debug!("Processing renamed file: {:?} -> {:?}", old_path, new_path);

                // Check if either path is an audio file
                let old_is_audio = is_audio_file(&old_path);
                let new_is_audio = is_audio_file(&new_path);

                if old_is_audio || new_is_audio {
                    // Trigger a scan to update paths
                    let stats = scanner.scan_source(source).await?;
                    debug!(
                        "Scan complete: {} new, {} updated, {} relocated",
                        stats.new_files, stats.updated_files, stats.relocated_files
                    );
                }
            }
        }

        Ok(())
    }
}

/// Convert a notify event to a WatcherEvent
fn convert_event(event: &Event) -> Option<WatcherEvent> {
    let paths = &event.paths;

    match &event.kind {
        EventKind::Create(_) => paths.first().map(|p| WatcherEvent::Created(p.clone())),
        EventKind::Modify(_) => paths.first().map(|p| WatcherEvent::Modified(p.clone())),
        EventKind::Remove(_) => paths.first().map(|p| WatcherEvent::Removed(p.clone())),
        EventKind::Other => {
            // Handle rename events which sometimes come as Other
            if paths.len() == 2 {
                Some(WatcherEvent::Renamed(paths[0].clone(), paths[1].clone()))
            } else {
                paths.first().map(|p| WatcherEvent::Modified(p.clone()))
            }
        }
        _ => None,
    }
}

/// Check if a path is an audio file based on extension
fn is_audio_file(path: &Path) -> bool {
    let audio_extensions = [
        "flac", "mp3", "m4a", "aac", "ogg", "opus", "wav", "aif", "aiff",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| audio_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Run the event processing loop
///
/// This function runs indefinitely, processing events as they arrive.
/// Call this in a separate task.
pub async fn run_event_loop(
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    mut event_rx: mpsc::Receiver<(i64, WatcherEvent)>,
) {
    let mut processor = EventProcessor::new(pool, user_id, device_id);

    // Flush interval (process pending events even if batch isn't full)
    let mut flush_interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            // Process incoming events
            Some((source_id, event)) = event_rx.recv() => {
                if let Err(e) = processor.process_event(source_id, event).await {
                    error!("Failed to process event: {}", e);
                }
            }
            // Periodic flush
            _ = flush_interval.tick() => {
                if let Err(e) = processor.flush_all().await {
                    error!("Failed to flush events: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("test.flac")));
        assert!(is_audio_file(Path::new("test.mp3")));
        assert!(is_audio_file(Path::new("test.FLAC")));
        assert!(is_audio_file(Path::new("/path/to/test.m4a")));
        assert!(!is_audio_file(Path::new("test.txt")));
        assert!(!is_audio_file(Path::new("test.jpg")));
        assert!(!is_audio_file(Path::new("test")));
    }

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert!(config.batch_events);
        assert_eq!(config.max_batch_size, 100);
    }

    #[test]
    fn test_convert_create_event() {
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/test/file.flac")],
            attrs: Default::default(),
        };

        let result = convert_event(&event);
        assert!(matches!(result, Some(WatcherEvent::Created(_))));
    }

    #[test]
    fn test_convert_modify_event() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![PathBuf::from("/test/file.flac")],
            attrs: Default::default(),
        };

        let result = convert_event(&event);
        assert!(matches!(result, Some(WatcherEvent::Modified(_))));
    }

    #[test]
    fn test_convert_remove_event() {
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![PathBuf::from("/test/file.flac")],
            attrs: Default::default(),
        };

        let result = convert_event(&event);
        assert!(matches!(result, Some(WatcherEvent::Removed(_))));
    }
}
