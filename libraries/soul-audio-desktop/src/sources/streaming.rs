//! Streaming audio source from server

use crossbeam_channel::{bounded, Receiver, Sender};
use soul_playback::{AudioSource, PlaybackError, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

const CHUNK_SIZE: usize = 8192; // Samples per chunk (4096 frames for stereo)
const BUFFER_CHUNKS: usize = 16; // Buffer up to 16 chunks

/// Audio source that streams from server
///
/// Downloads audio chunks in background while playback continues.
/// Uses buffering to handle network latency.
pub struct StreamingAudioSource {
    /// URL of the audio stream
    url: String,

    /// Sample rate
    sample_rate: u32,

    /// Number of channels
    channels: u16,

    /// Total duration
    duration: Duration,

    /// Current read position in samples
    position: usize,

    /// Buffer of decoded samples (shared with download thread)
    buffer: Arc<Mutex<Vec<f32>>>,

    /// Channel to receive new chunks from download thread
    chunk_receiver: Receiver<Vec<f32>>,

    /// Flag to signal download thread to stop
    stop_signal: Arc<AtomicBool>,

    /// Handle to background download thread
    _download_thread: Option<thread::JoinHandle<()>>,

    /// Whether stream has finished
    finished: bool,

    /// Whether we've encountered an error
    error: Arc<Mutex<Option<String>>>,
}

impl StreamingAudioSource {
    /// Create a new streaming audio source
    ///
    /// Starts background download thread that fetches audio chunks from the server.
    ///
    /// # Arguments
    /// * `url` - URL of the audio stream endpoint
    /// * `sample_rate` - Sample rate of the audio
    /// * `channels` - Number of audio channels (1=mono, 2=stereo)
    /// * `duration` - Total duration of the track
    ///
    /// # Returns
    /// * `Ok(source)` - Streaming source ready for playback
    /// * `Err(_)` - Failed to initialize stream
    pub fn new(url: String, sample_rate: u32, channels: u16, duration: Duration) -> Result<Self> {
        let (chunk_sender, chunk_receiver) = bounded(BUFFER_CHUNKS);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));

        // Start background download thread
        let download_url = url.clone();
        let stop_signal_clone = Arc::clone(&stop_signal);
        let error_clone = Arc::clone(&error);
        let download_thread = thread::spawn(move || {
            Self::download_stream(download_url, chunk_sender, stop_signal_clone, error_clone);
        });

        Ok(Self {
            url,
            sample_rate,
            channels,
            duration,
            position: 0,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(CHUNK_SIZE * BUFFER_CHUNKS))),
            chunk_receiver,
            stop_signal,
            _download_thread: Some(download_thread),
            finished: false,
            error,
        })
    }

    /// Background thread to download and decode stream
    ///
    /// This runs in a separate thread and downloads audio chunks from the server,
    /// sending them to the main playback thread via channel.
    fn download_stream(
        url: String,
        sender: Sender<Vec<f32>>,
        stop_signal: Arc<AtomicBool>,
        error: Arc<Mutex<Option<String>>>,
    ) {
        // Create a tokio runtime for async HTTP operations
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                *error.lock().unwrap() = Some(format!("Failed to create runtime: {}", e));
                return;
            }
        };

        // Run async download in the runtime
        runtime.block_on(async {
            if let Err(e) = Self::download_stream_async(&url, &sender, &stop_signal).await {
                *error.lock().unwrap() = Some(format!("Streaming error: {}", e));
            }
        });
    }

    /// Async implementation of stream downloading
    async fn download_stream_async(
        url: &str,
        sender: &Sender<Vec<f32>>,
        stop_signal: &Arc<AtomicBool>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::AsyncReadExt;

        // Create HTTP client
        let client = reqwest::Client::new();

        // Start streaming request
        let mut response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        // Buffer for reading bytes
        let mut byte_buffer = vec![0u8; CHUNK_SIZE * 4]; // f32 = 4 bytes

        // Stream chunks
        while !stop_signal.load(Ordering::Relaxed) {
            // Read chunk of bytes
            let bytes_read = match response.chunk().await? {
                Some(chunk) => {
                    let len = chunk.len().min(byte_buffer.len());
                    byte_buffer[..len].copy_from_slice(&chunk[..len]);
                    len
                }
                None => break, // End of stream
            };

            if bytes_read == 0 {
                break;
            }

            // Convert bytes to f32 samples (assuming little-endian f32)
            let sample_count = bytes_read / 4;
            let mut samples = Vec::with_capacity(sample_count);

            for i in 0..sample_count {
                let offset = i * 4;
                if offset + 4 <= bytes_read {
                    let bytes = [
                        byte_buffer[offset],
                        byte_buffer[offset + 1],
                        byte_buffer[offset + 2],
                        byte_buffer[offset + 3],
                    ];
                    samples.push(f32::from_le_bytes(bytes));
                }
            }

            // Send samples to playback thread
            if sender.send(samples).is_err() {
                break; // Receiver dropped
            }
        }

        Ok(())
    }

    /// Fill internal buffer from received chunks
    ///
    /// Pulls all available chunks from the channel into the shared buffer.
    fn fill_buffer(&mut self) -> Result<()> {
        let mut buffer = self.buffer.lock().unwrap();

        // Try to receive all available chunks
        while let Ok(chunk) = self.chunk_receiver.try_recv() {
            buffer.extend(chunk);
        }

        // Check for errors from download thread
        if let Some(err_msg) = self.error.lock().unwrap().take() {
            return Err(PlaybackError::AudioSource(err_msg));
        }

        Ok(())
    }

    /// Get current buffer length
    fn buffer_len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }
}

impl AudioSource for StreamingAudioSource {
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize> {
        // Fill internal buffer from network
        self.fill_buffer()?;

        let mut buffer = self.buffer.lock().unwrap();

        // Calculate how many samples we can read
        let available = buffer.len().saturating_sub(self.position);

        if available == 0 {
            // Check if stream is finished
            drop(buffer); // Release lock before checking channel
            if self.chunk_receiver.is_empty() {
                self.finished = true;
                return Ok(0);
            }

            // Buffer underrun - return silence and hope more data arrives soon
            output.fill(0.0);
            return Ok(0);
        }

        let to_read = available.min(output.len());

        // Copy from buffer
        output[..to_read].copy_from_slice(&buffer[self.position..self.position + to_read]);

        // Update position
        self.position += to_read;

        // Remove consumed samples from buffer to prevent unbounded growth
        if self.position > CHUNK_SIZE * 4 {
            buffer.drain(0..self.position);
            self.position = 0;
        }

        Ok(to_read)
    }

    fn seek(&mut self, _position: Duration) -> Result<()> {
        // Seeking in streaming is complex and would require:
        // 1. Stopping current download thread
        // 2. Requesting new stream from specific position
        // 3. Clearing buffer
        // 4. Starting download from new position
        //
        // This is not implemented in MVP - streaming is assumed to be
        // sequential playback only.
        Err(PlaybackError::InvalidOperation(
            "Seeking not supported for streaming sources".to_string(),
        ))
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        // Calculate position from samples read
        let frames = self.position / self.channels as usize;
        Duration::from_secs_f64(frames as f64 / self.sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}

impl StreamingAudioSource {
    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

impl Drop for StreamingAudioSource {
    fn drop(&mut self) {
        // Signal download thread to stop
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_source_implements_audio_source() {
        // This test ensures the trait is implemented
        fn assert_audio_source<T: AudioSource>() {}
        assert_audio_source::<StreamingAudioSource>();
    }

    #[test]
    fn create_streaming_source() {
        let source = StreamingAudioSource::new(
            "http://localhost:8080/stream/track1".to_string(),
            44100,
            2, // stereo
            Duration::from_secs(180),
        );

        assert!(source.is_ok());
        let source = source.unwrap();
        assert_eq!(source.sample_rate(), 44100);
        assert_eq!(source.channels(), 2);
    }
}
