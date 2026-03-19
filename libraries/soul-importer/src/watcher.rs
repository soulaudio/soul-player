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
use std::collections::{HashMap, HashSet};
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

/// Scan lifecycle events emitted by the watcher's event processor.
/// Platform-agnostic — the Tauri layer converts these to frontend events.
#[derive(Debug, Clone)]
pub enum ScanEvent {
    Started,
    Progress {
        processed: i64,
        total: i64,
        current_file: Option<String>,
    },
    Complete,
}

/// Callback for scan lifecycle events (platform-agnostic)
pub type ScanEventCallback = Arc<dyn Fn(ScanEvent) + Send + Sync>;

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

        // Verify path exists (async to avoid blocking on network/slow storage)
        if !tokio::fs::try_exists(source_path).await.unwrap_or(false) {
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

/// Event processor that handles watcher events and updates the library.
///
/// Batches events per source and collapses them into a single incremental
/// scan per flush. With directory-level mtime skipping, a full source scan
/// is fast for unchanged directories.
pub struct EventProcessor {
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    /// Pending events batched by source ID
    pending: HashMap<i64, Vec<WatcherEvent>>,
    /// Maximum batch size before forcing a scan
    max_batch_size: usize,
    /// Sources currently being scanned (prevents overlapping scans)
    scanning: HashSet<i64>,
    /// Optional callback for scan lifecycle events
    scan_callback: Option<ScanEventCallback>,
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
            scanning: HashSet::new(),
            scan_callback: None,
        }
    }

    /// Set the maximum batch size
    pub fn max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Set callback for scan lifecycle events
    pub fn on_scan_event(mut self, callback: ScanEventCallback) -> Self {
        self.scan_callback = Some(callback);
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

    /// Flush pending events for a source by triggering a single incremental scan.
    ///
    /// All batched events are collapsed — with directory-level mtime skipping,
    /// one scan handles everything efficiently.
    pub async fn flush_source(&mut self, source_id: i64) -> Result<()> {
        let events = match self.pending.remove(&source_id) {
            Some(events) if !events.is_empty() => events,
            _ => return Ok(()),
        };

        // Skip if already scanning this source — re-queue events
        if self.scanning.contains(&source_id) {
            debug!(
                "Source {} already scanning, re-queuing {} events",
                source_id,
                events.len()
            );
            self.pending.insert(source_id, events);
            return Ok(());
        }

        info!(
            "Watcher triggering scan for source {} ({} events collapsed)",
            source_id,
            events.len()
        );

        let source = soul_storage::library_sources::get_by_id(&self.pool, source_id).await?;
        let Some(source) = source else {
            warn!("Source {} not found, skipping events", source_id);
            return Ok(());
        };

        self.scanning.insert(source_id);

        // Build scanner with progress callback if available
        let mut scanner = LibraryScanner::new(
            self.pool.clone(),
            self.user_id.clone(),
            self.device_id.clone(),
        );

        if let Some(ref cb) = self.scan_callback {
            let cb = cb.clone();
            scanner = scanner.on_progress(Box::new(move |stats| {
                cb(ScanEvent::Progress {
                    processed: stats.processed,
                    total: stats.total_files,
                    current_file: stats.current_file.clone(),
                });
            }));
        }

        if let Some(ref cb) = self.scan_callback {
            cb(ScanEvent::Started);
        }

        match scanner.scan_source(&source).await {
            Ok(stats) => {
                info!(
                    "Watcher scan complete for {}: {} new, {} updated, {} removed",
                    source.name, stats.new_files, stats.updated_files, stats.removed_files
                );
            }
            Err(e) => {
                error!("Watcher scan failed for {}: {}", source.name, e);
            }
        }

        if let Some(ref cb) = self.scan_callback {
            cb(ScanEvent::Complete);
        }

        self.scanning.remove(&source_id);
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

/// Run the event processing loop
///
/// This function runs indefinitely, processing events as they arrive.
/// Call this in a separate task.
pub async fn run_event_loop(
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    mut event_rx: mpsc::Receiver<(i64, WatcherEvent)>,
    scan_callback: Option<ScanEventCallback>,
) {
    let mut processor = EventProcessor::new(pool, user_id, device_id);
    if let Some(cb) = scan_callback {
        processor = processor.on_scan_event(cb);
    }

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
        // Use the shared scanner function
        assert!(crate::scanner::is_audio_file(Path::new("test.flac")));
        assert!(crate::scanner::is_audio_file(Path::new("test.mp3")));
        assert!(crate::scanner::is_audio_file(Path::new("test.FLAC")));
        assert!(crate::scanner::is_audio_file(Path::new("/path/to/test.m4a")));
        assert!(!crate::scanner::is_audio_file(Path::new("test.txt")));
        assert!(!crate::scanner::is_audio_file(Path::new("test.jpg")));
        assert!(!crate::scanner::is_audio_file(Path::new("test")));
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
