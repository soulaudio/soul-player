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
    /// Handle to the loader thread
    _thread_handle: JoinHandle<()>,
    /// Flag to signal shutdown
    shutdown: Arc<Mutex<bool>>,
}

impl TrackLoader {
    /// Create a new track loader with a background thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<LoadRequest>(4);
        let (result_tx, result_rx) = bounded::<LoadResult>(4);
        let shutdown = Arc::new(Mutex::new(false));
        let shutdown_clone = shutdown.clone();

        let thread_handle = thread::Builder::new()
            .name("track-loader".to_string())
            .spawn(move || {
                Self::loader_thread(request_rx, result_tx, shutdown_clone);
            })
            .expect("Failed to spawn track loader thread");

        Self {
            request_tx,
            result_rx,
            _thread_handle: thread_handle,
            shutdown,
        }
    }

    /// Request loading a track (non-blocking)
    ///
    /// Returns true if the request was queued, false if the queue is full.
    pub fn request_load(&self, request: LoadRequest) -> bool {
        match self.request_tx.try_send(request) {
            Ok(()) => true,
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                eprintln!("[TrackLoader] Load request queue full, dropping request");
                false
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                eprintln!("[TrackLoader] Load request channel disconnected");
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
                eprintln!("[TrackLoader] Result channel disconnected");
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
    pub fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
        // Send a dummy request to wake up the thread if it's waiting
        // (The thread will check the shutdown flag and exit)
    }

    /// Background thread that handles load requests
    fn loader_thread(
        request_rx: Receiver<LoadRequest>,
        result_tx: Sender<LoadResult>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        eprintln!("[TrackLoader] Background thread started");

        loop {
            // Check for shutdown
            if *shutdown.lock().unwrap() {
                eprintln!("[TrackLoader] Shutdown requested, exiting");
                break;
            }

            // Wait for a load request (with timeout to allow shutdown checks)
            match request_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(request) => {
                    let start = std::time::Instant::now();
                    eprintln!(
                        "[TrackLoader] Loading track: {} (preload: {})",
                        request.track.title, request.is_preload
                    );

                    // This is the slow part - disk I/O!
                    let result = match LocalAudioSource::new(&request.path, request.target_sample_rate)
                    {
                        Ok(source) => {
                            let duration = start.elapsed();
                            eprintln!(
                                "[TrackLoader] Loaded '{}' in {}ms",
                                request.track.title,
                                duration.as_millis()
                            );
                            LoadResult {
                                source: Some(Box::new(source)),
                                track: request.track,
                                error: None,
                                is_preload: request.is_preload,
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[TrackLoader] Failed to load '{}': {}",
                                request.track.title, e
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
                        eprintln!("[TrackLoader] Failed to send load result, channel closed");
                        break;
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No request, continue loop (will check shutdown flag)
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    eprintln!("[TrackLoader] Request channel disconnected, exiting");
                    break;
                }
            }
        }

        eprintln!("[TrackLoader] Background thread exiting");
    }
}

impl Default for TrackLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TrackLoader {
    fn drop(&mut self) {
        self.shutdown();
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

        let loader = TrackLoader::new();

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
        let mut result = None;
        for _ in 0..100 {
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
        let loader = TrackLoader::new();

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
        let loader = TrackLoader::new();

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
}
