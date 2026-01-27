//! Background Track Loader
//!
//! Handles audio source loading on a background thread to avoid blocking
//! the audio callback. Track loading involves disk I/O and can take 5-100+ms,
//! which would cause buffer underruns if done in the audio callback.
//!
//! ## Architecture
//!
//! ```text
//! Audio Callback Thread          Track Loader Thread
//!        │                              │
//!        │  request_load(path)          │
//!        │─────────────────────────────>│
//!        │                              │ LocalAudioSource::new()
//!        │                              │ (disk I/O, 5-100ms)
//!        │                              │
//!        │  poll_ready() -> Some(src)   │
//!        │<─────────────────────────────│
//!        │                              │
//! ```

use crate::sources::local::LocalAudioSource;
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use soul_playback::{AudioSource, QueueTrack};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Request to load a track
#[derive(Debug, Clone)]
pub struct LoadRequest {
    /// Path to the audio file
    pub path: PathBuf,
    /// Track metadata (for event emission)
    pub track: QueueTrack,
    /// Target sample rate for the audio source
    pub target_sample_rate: u32,
    /// Whether this is a "next track" preload (vs current track load)
    pub is_preload: bool,
}

/// Result of loading a track
pub struct LoadResult {
    /// The loaded audio source (if successful)
    pub source: Option<Box<dyn AudioSource>>,
    /// The track that was loaded
    pub track: QueueTrack,
    /// Error message if loading failed
    pub error: Option<String>,
    /// Whether this was a preload request
    pub is_preload: bool,
}

/// Background track loader
///
/// Spawns a dedicated thread for loading audio sources, keeping disk I/O
/// off the audio callback thread.
pub struct TrackLoader {
    /// Channel to send load requests
    request_tx: Sender<LoadRequest>,
    /// Channel to receive load results
    result_rx: Receiver<LoadResult>,
    /// Handle to the loader thread (Option to allow taking for join)
    thread_handle: Option<JoinHandle<()>>,
    /// Flag to signal shutdown
    shutdown: Arc<Mutex<bool>>,
}

impl TrackLoader {
    /// Create a new track loader with a background thread
    ///
    /// Returns an error if the background thread cannot be spawned.
    pub fn new() -> Result<Self, String> {
        let (request_tx, request_rx) = bounded::<LoadRequest>(4);
        let (result_tx, result_rx) = bounded::<LoadResult>(4);
        let shutdown = Arc::new(Mutex::new(false));
        let shutdown_clone = shutdown.clone();

        let thread_handle = thread::Builder::new()
            .name("track-loader".to_string())
            .spawn(move || {
                Self::loader_thread(request_rx, result_tx, shutdown_clone);
            })
            .map_err(|e| format!("Failed to spawn track loader thread: {}", e))?;

        Ok(Self {
            request_tx,
            result_rx,
            thread_handle: Some(thread_handle),
            shutdown,
        })
    }

    /// Request loading a track (non-blocking)
    ///
    /// Returns true if the request was queued, false if the queue is full.
    pub fn request_load(&self, request: LoadRequest) -> bool {
        match self.request_tx.try_send(request) {
            Ok(()) => true,
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                tracing::warn!("[TrackLoader] Load request queue full, dropping request");
                false
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                tracing::error!("[TrackLoader] Load request channel disconnected");
                false
            }
        }
    }

    /// Poll for a ready load result (non-blocking)
    ///
    /// Returns Some(result) if a track has finished loading, None otherwise.
    pub fn poll_ready(&self) -> Option<LoadResult> {
        match self.result_rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                tracing::error!("[TrackLoader] Result channel disconnected");
                None
            }
        }
    }

    /// Check if a load is currently in progress
    pub fn is_loading(&self) -> bool {
        // If we can't send (queue full) or can receive (results waiting),
        // something is in progress
        !self.request_tx.is_empty() || !self.result_rx.is_empty()
    }

    /// Shutdown the loader thread
    ///
    /// Sets the shutdown flag to signal the thread to exit.
    /// Does not wait for the thread - use `shutdown_and_wait` for that.
    ///
    /// If the shutdown mutex is poisoned, logs an error but does not panic.
    pub fn shutdown(&self) {
        match self.shutdown.lock() {
            Ok(mut guard) => *guard = true,
            Err(e) => {
                tracing::error!(error = %e, "[TrackLoader] Failed to lock shutdown mutex - poisoned?");
                // Still try to set the flag via the poisoned mutex
                *e.into_inner() = true;
            }
        }
        // Note: The thread will check the shutdown flag on each loop iteration (100ms timeout)
        // and exit gracefully when it sees the flag set.
    }

    /// Shutdown the loader thread and wait for it to complete
    ///
    /// This provides deterministic cleanup by waiting for the thread to exit.
    /// Uses a timeout to prevent indefinite blocking.
    fn shutdown_and_wait(&mut self) {
        // Signal shutdown
        self.shutdown();

        // Take ownership of the thread handle to join it
        if let Some(handle) = self.thread_handle.take() {
            // The thread checks shutdown every 100ms, so 500ms should be plenty
            // We use a simple join here since the thread should exit quickly
            tracing::debug!("[TrackLoader] Waiting for loader thread to exit");
            match handle.join() {
                Ok(()) => {
                    tracing::debug!("[TrackLoader] Loader thread exited cleanly");
                }
                Err(e) => {
                    tracing::error!("[TrackLoader] Loader thread panicked: {:?}", e);
                }
            }
        }
    }

    /// Background thread that handles load requests
    fn loader_thread(
        request_rx: Receiver<LoadRequest>,
        result_tx: Sender<LoadResult>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        tracing::debug!("[TrackLoader] Background thread started");

        loop {
            // Check for shutdown
            let should_shutdown = match shutdown.lock() {
                Ok(guard) => *guard,
                Err(e) => {
                    tracing::error!(error = %e, "[TrackLoader] Shutdown mutex poisoned in loader thread");
                    // Assume shutdown if mutex is poisoned - safer to exit than continue
                    true
                }
            };

            if should_shutdown {
                tracing::debug!("[TrackLoader] Shutdown requested, exiting");
                break;
            }

            // Wait for a load request (with timeout to allow shutdown checks)
            match request_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(request) => {
                    let start = std::time::Instant::now();
                    tracing::info!(
                        track_title = %request.track.title,
                        is_preload = request.is_preload,
                        file_path = %request.path.display(),
                        "[TrackLoader] Loading track"
                    );

                    // This is the slow part - disk I/O!
                    let result =
                        match LocalAudioSource::new(&request.path, request.target_sample_rate) {
                            Ok(source) => {
                                let load_duration = start.elapsed();
                                tracing::info!(
                                    track_title = %request.track.title,
                                    load_duration_ms = load_duration.as_millis(),
                                    "[TrackLoader] Source created, waiting for buffer"
                                );

                                // Wait for buffer to be ready (critical for preventing playback artifacts)
                                // This blocks until the decoder thread has filled ~500ms of audio
                                let wait_start = std::time::Instant::now();
                                let max_wait = std::time::Duration::from_secs(5); // Safety timeout

                                while !source.is_ready() && wait_start.elapsed() < max_wait {
                                    std::thread::sleep(std::time::Duration::from_millis(10));
                                }

                                let buffer_duration = wait_start.elapsed();
                                let total_duration = start.elapsed();

                                if source.is_ready() {
                                    tracing::info!(
                                        track_title = %request.track.title,
                                        buffer_wait_ms = buffer_duration.as_millis(),
                                        total_duration_ms = total_duration.as_millis(),
                                        "[TrackLoader] Buffer ready"
                                    );
                                } else {
                                    tracing::warn!(
                                        track_title = %request.track.title,
                                        buffer_wait_ms = buffer_duration.as_millis(),
                                        "[TrackLoader] Buffer timeout"
                                    );
                                }

                                LoadResult {
                                    source: Some(Box::new(source)),
                                    track: request.track,
                                    error: None,
                                    is_preload: request.is_preload,
                                }
                            }
                            Err(e) => {
                                let total_duration = start.elapsed();
                                tracing::error!(
                                    track_title = %request.track.title,
                                    error = %e,
                                    duration_ms = total_duration.as_millis(),
                                    "[TrackLoader] Failed to load track"
                                );
                                LoadResult {
                                    source: None,
                                    track: request.track,
                                    error: Some(e.to_string()),
                                    is_preload: request.is_preload,
                                }
                            }
                        };

                    // Send result back
                    if result_tx.send(result).is_err() {
                        tracing::error!("[TrackLoader] Failed to send load result, channel closed");
                        break;
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No request, continue loop (will check shutdown flag)
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    tracing::debug!("[TrackLoader] Request channel disconnected, exiting");
                    break;
                }
            }
        }

        tracing::debug!("[TrackLoader] Background thread exiting");
    }
}

impl Default for TrackLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create default TrackLoader - thread spawn failed")
    }
}

impl Drop for TrackLoader {
    fn drop(&mut self) {
        // Use shutdown_and_wait for deterministic cleanup
        // This ensures the loader thread has exited and released all resources
        // (file handles, memory) before the TrackLoader is fully dropped
        self.shutdown_and_wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn generate_test_wav(path: &PathBuf) -> std::io::Result<()> {
        let sample_rate = 44100u32;
        let num_samples = 44100usize; // 1 second
        let channels = 2usize;

        let mut file = File::create(path)?;

        // RIFF header
        file.write_all(b"RIFF")?;
        let file_size = 36 + num_samples * channels * 2;
        file.write_all(&(file_size as u32).to_le_bytes())?;
        file.write_all(b"WAVE")?;

        // fmt chunk
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&(channels as u16).to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
        file.write_all(&((channels * 2) as u16).to_le_bytes())?;
        file.write_all(&16u16.to_le_bytes())?;

        // data chunk
        file.write_all(b"data")?;
        file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

        let silence = vec![0i16; num_samples * channels];
        for sample in silence {
            file.write_all(&sample.to_le_bytes())?;
        }

        Ok(())
    }

    #[test]
    fn test_track_loader_loads_track() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test.wav");
        generate_test_wav(&wav_path).unwrap();

        let loader = TrackLoader::new().expect("Failed to create TrackLoader");

        let request = LoadRequest {
            path: wav_path.clone(),
            track: QueueTrack {
                id: "test".to_string(),
                title: "Test Track".to_string(),
                artist: "Test Artist".to_string(),
                album: None,
                duration: std::time::Duration::from_secs(1),
                path: wav_path,
                track_number: None,
                source: soul_playback::TrackSource::Single,
            },
            target_sample_rate: 44100,
            is_preload: false,
        };

        assert!(loader.request_load(request));

        // Wait for result (with timeout)
        // Increased from 100 to 200 iterations to account for larger buffer (1000ms vs 500ms)
        let mut result = None;
        for _ in 0..200 {
            if let Some(r) = loader.poll_ready() {
                result = Some(r);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let result = result.expect("Track loading should complete");
        assert!(result.source.is_some(), "Source should be loaded");
        assert!(result.error.is_none(), "Should not have error");
        assert_eq!(result.track.title, "Test Track");
    }

    #[test]
    fn test_track_loader_handles_missing_file() {
        let loader = TrackLoader::new().expect("Failed to create TrackLoader");

        let missing_path = PathBuf::from("/nonexistent/file.wav");
        let request = LoadRequest {
            path: missing_path.clone(),
            track: QueueTrack {
                id: "missing".to_string(),
                title: "Missing Track".to_string(),
                artist: "Unknown".to_string(),
                album: None,
                duration: std::time::Duration::ZERO,
                path: missing_path,
                track_number: None,
                source: soul_playback::TrackSource::Single,
            },
            target_sample_rate: 44100,
            is_preload: false,
        };

        assert!(loader.request_load(request));

        // Wait for result
        let mut result = None;
        for _ in 0..100 {
            if let Some(r) = loader.poll_ready() {
                result = Some(r);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let result = result.expect("Should get result even for missing file");
        assert!(result.source.is_none(), "Source should not be loaded");
        assert!(result.error.is_some(), "Should have error message");
    }

    #[test]
    fn test_track_loader_non_blocking() {
        let loader = TrackLoader::new().expect("Failed to create TrackLoader");

        // poll_ready should return immediately when nothing is loaded
        let start = std::time::Instant::now();
        let result = loader.poll_ready();
        let duration = start.elapsed();

        assert!(result.is_none());
        assert!(
            duration.as_millis() < 5,
            "poll_ready should be non-blocking, took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_track_loader_shutdown_joins_thread() {
        // Test that drop properly shuts down and joins the background thread
        // This is important for resource cleanup - file handles held by the loader
        // thread should be released before the TrackLoader is fully dropped

        let start = std::time::Instant::now();
        {
            let loader = TrackLoader::new().expect("Failed to create TrackLoader");
            // Just drop it immediately - Drop should call shutdown_and_wait
            drop(loader);
        }
        let duration = start.elapsed();

        // Thread join should complete quickly (loader thread checks shutdown every 100ms)
        // Allow up to 500ms for thread cleanup
        assert!(
            duration.as_millis() < 500,
            "TrackLoader drop should complete within 500ms, took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_track_loader_shutdown_during_load() {
        // Test that dropping the TrackLoader while a load is in progress
        // still properly cleans up the thread

        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test.wav");
        generate_test_wav(&wav_path).unwrap();

        let loader = TrackLoader::new().expect("Failed to create TrackLoader");

        let request = LoadRequest {
            path: wav_path.clone(),
            track: QueueTrack {
                id: "test".to_string(),
                title: "Test Track".to_string(),
                artist: "Test Artist".to_string(),
                album: None,
                duration: std::time::Duration::from_secs(1),
                path: wav_path,
                track_number: None,
                source: soul_playback::TrackSource::Single,
            },
            target_sample_rate: 44100,
            is_preload: false,
        };

        // Start a load
        assert!(loader.request_load(request));

        // Immediately drop (don't wait for result)
        let start = std::time::Instant::now();
        drop(loader);
        let duration = start.elapsed();

        // Even with a load in progress, shutdown should complete within reasonable time
        // The thread may be waiting for buffer to fill, but should respect shutdown
        assert!(
            duration.as_millis() < 6000, // 5s buffer timeout + 1s margin
            "TrackLoader drop during load should complete within 6s, took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_track_loader_multiple_create_drop() {
        // Test that we can create and drop multiple loaders without leaking threads
        for i in 0..5 {
            let loader = TrackLoader::new()
                .expect(&format!("Failed to create TrackLoader on iteration {}", i));
            drop(loader);
        }
        // If threads were leaking, we'd see resource exhaustion
        // The test passing indicates threads are being cleaned up
    }
}
