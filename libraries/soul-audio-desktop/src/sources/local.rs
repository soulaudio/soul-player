//! Local file audio source using Symphonia decoder with streaming
//!
//! # Format Support
//!
//! This module provides universal audio format support through Symphonia,
//! with an abstract conversion layer that handles all sample formats uniformly.
//!
//! ## Supported Formats
//! - **Containers**: MP3, FLAC, OGG, WAV, AAC, OPUS, M4A, etc.
//! - **Sample types**: All Symphonia formats (F32, F64, S8, S16, S24, S32, U8, U16, U24, U32)
//! - **Channel layouts**: Mono (duplicated to stereo), Stereo, Multi-channel (mixed to stereo)
//!
//! ## Architecture
//!
//! The design uses a background decoder thread to avoid blocking the audio callback:
//!
//! ```text
//! Audio Callback Thread          Decoder Thread
//!        │                              │
//!        │  read_samples()              │ decode_next_packet()
//!        │  (non-blocking read)         │ (disk I/O, resampling)
//!        │       ▲                      │      │
//!        │       └──────────────────────┴──────┘
//!        │            output_buffer (Arc<Mutex<>>)
//! ```
//!
//! 1. **Background Decoder Thread**:
//!    - Continuously decodes packets and fills `output_buffer`
//!    - Handles all disk I/O and resampling off the audio thread
//!    - Keeps buffer at ~5 seconds of audio
//!
//! 2. **Non-blocking `read_samples()`**:
//!    - Only reads from `output_buffer` (no decoding)
//!    - Returns available samples or silence if buffer empty
//!    - Never blocks on disk I/O
//!
//! 3. **Format Conversion** (`convert_to_f32_interleaved`):
//!    - Handles all Symphonia sample formats
//!    - Normalizes to [-1.0, 1.0] range

use crossbeam_channel::{bounded, Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use soul_playback::{AudioSource, PlaybackError, Result};
use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;
use tracing;

/// Size of ring buffer in seconds
const BUFFER_SIZE_SECONDS: usize = 5;

/// Minimum buffer level (in samples) before source is considered ready for playback
/// At stereo 48kHz, 24000 samples = 250ms of audio (fast start, 5s background buffer for safety)
///
/// This MUST be filled BEFORE playback starts to prevent buffer underrun.
/// The user reported jitter on first play but not on "previous track" because:
/// - First play: file not cached → slow disk I/O → buffer underrun
/// - Previous track: file cached → fast I/O → no underrun
///
/// Prebuffer before playback starts.
///
/// Trade-off:
/// - Higher = more safety margin for slow disks, but higher latency
/// - Lower = faster start, but risk of underrun on slow systems
///
/// 12000 samples = ~125ms at 48kHz stereo, which provides:
/// - Very fast perceived response (< 200ms feels instant)
/// - Enough buffer for smooth startup
/// - The 5-second background buffer provides long-term safety
const MIN_BUFFER_SAMPLES: usize = 12000;

/// Encoder delay values by codec type (in frames, not samples)
///
/// Most audio codecs add "encoder delay" - padding samples at file start:
/// - MP3: ~1152 frames (26ms @ 44.1kHz)
/// - AAC: ~1024-2112 frames
/// - FLAC: ~0 frames (lossless, no encoder delay)
/// - WAV/PCM: ~0 frames (raw audio)
///
/// These samples contain codec startup artifacts (DC offset, filter ramp-up,
/// near-silence with sudden jumps) that cause audible pops if played directly.
///
/// CRITICAL: Using wrong values causes audible issues:
/// - Too high (like 1200 for FLAC): Skips actual music, causes stuttering/repeats
/// - Too low: Pops and clicks at playback start
fn get_encoder_delay_frames(codec_params: &symphonia::core::codecs::CodecParameters) -> usize {
    // Check codec type from CodecParameters
    use symphonia::core::codecs::CODEC_TYPE_AAC;
    use symphonia::core::codecs::CODEC_TYPE_FLAC;
    use symphonia::core::codecs::CODEC_TYPE_MP3;
    use symphonia::core::codecs::CODEC_TYPE_PCM_F32LE;
    use symphonia::core::codecs::CODEC_TYPE_PCM_S16LE;
    use symphonia::core::codecs::CODEC_TYPE_PCM_S24LE;

    match codec_params.codec {
        CODEC_TYPE_MP3 => 1152, // Standard MP3 encoder delay
        CODEC_TYPE_AAC => 1024, // Typical AAC encoder delay
        CODEC_TYPE_FLAC => 0,   // FLAC is lossless, no encoder delay
        CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S24LE | CODEC_TYPE_PCM_F32LE => 0, // Raw PCM
        _ => 512,               // Conservative default for unknown codecs (Vorbis, Opus, etc.)
    }
}

/// Commands sent to the decoder thread
#[derive(Debug)]
enum DecoderCommand {
    /// Seek to a position
    Seek(Duration),
    /// Stop decoding and exit thread
    Stop,
}

/// Shared state between audio thread and decoder thread
struct SharedState {
    /// Total samples read by audio callback (at target rate)
    samples_read: AtomicUsize,
    /// Whether decoder has reached end of file
    is_eof: AtomicBool,
    /// Whether a seek is pending (decoder will reset)
    seek_pending: AtomicBool,
    /// Generation counter - incremented on each seek to invalidate stale ring buffer samples
    /// This solves the 3-5 second audio delay by skipping old samples after seek
    generation: AtomicUsize,
}

/// Audio source for local files with background decoder thread
///
/// Uses Symphonia to decode audio files from disk in a background thread.
/// The audio callback only reads from a pre-filled buffer, never blocking on I/O.
/// Maintains a 5-second lock-free ring buffer for smooth, glitch-free playback.
/// Automatically resamples audio to match target sample rate.
///
/// Supports all formats: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct LocalAudioSource {
    path: PathBuf,
    source_sample_rate: u32, // Sample rate of the audio file
    target_sample_rate: u32, // Target output sample rate
    channels: u16,

    // Lock-free ring buffer (consumer side for audio callback)
    buffer_consumer: Consumer<f32>,

    // Shared state with decoder thread (lock-free atomics)
    shared: Arc<SharedState>,
    output_buffer_capacity: usize, // Max samples to buffer

    // Communication with decoder thread
    command_tx: Sender<DecoderCommand>,
    _decoder_thread: JoinHandle<()>,

    // Position tracking
    total_duration: Duration,
    needs_resampling: bool,

    // Track last seen generation to detect seeks and drain stale buffer
    last_generation: usize,
}

// ===== Helper Functions for Decoder Thread =====

/// Skip encoder delay samples from decoded audio
///
/// Encoder delay (codec startup artifacts) causes pops if played directly.
/// This function skips the first ENCODER_DELAY_FRAMES worth of samples.
///
/// # Arguments
/// * `samples` - Decoded samples to process
/// * `skip_target` - Total samples to skip
/// * `samples_skipped` - Current skip counter
/// * `producer` - Ring buffer producer for output
///
/// # Returns
/// Updated skip counter
fn skip_encoder_delay(
    samples: Vec<f32>,
    skip_target: usize,
    mut samples_skipped: usize,
    producer: &mut Producer<f32>,
) -> usize {
    for sample in samples {
        if samples_skipped < skip_target {
            samples_skipped += 1;
            continue; // Drop sample
        }
        // Non-blocking push - if buffer is full, skip the sample
        let _ = producer.push(sample);
    }
    samples_skipped
}

/// Handle seek command - reset decoder and buffers
///
/// Performs seek operation and resets all decoder state.
/// Returns reset skip counters (resampler_skip, encoder_delay_skip).
fn handle_seek_command(
    format_reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::Decoder>,
    resampler: &mut Option<SincFixedIn<f32>>,
    input_buffer: &mut VecDeque<f32>,
    _producer: &mut Producer<f32>,
    shared: &Arc<SharedState>,
    position: Duration,
    track_id: u32,
    time_base: TimeBase,
    target_sample_rate: u32,
    channels: u16,
) -> std::result::Result<(usize, usize), String> {
    // Perform seek using Time mode for better accuracy with VBR files
    let secs = position.as_secs();
    let frac = (position.as_nanos() % 1_000_000_000) as f64 / 1_000_000_000.0;
    let time = symphonia::core::units::Time::new(secs, frac);

    let seeked_to = format_reader
        .seek(
            symphonia::core::formats::SeekMode::Accurate,
            symphonia::core::formats::SeekTo::Time {
                time,
                track_id: Some(track_id),
            },
        )
        .map_err(|e| format!("Seek failed: {}", e))?;

    // Reset decoder state
    decoder.reset();
    input_buffer.clear();
    if let Some(ref mut r) = resampler {
        r.reset();
    }

    // Calculate actual position from seek result using time_base
    // This is crucial for VBR files where the actual position may differ from requested
    let actual_position = Duration::from_secs_f64(
        seeked_to.actual_ts as f64 * time_base.numer as f64 / time_base.denom as f64,
    );

    // Log seek accuracy for debugging VBR issues
    let seek_error_ms = actual_position
        .saturating_sub(position)
        .max(position.saturating_sub(actual_position))
        .as_millis();

    if seek_error_ms > 50 {
        tracing::warn!(
            requested_ms = position.as_millis(),
            actual_ms = actual_position.as_millis(),
            error_ms = seek_error_ms,
            "[DecoderThread] Seek accuracy issue: landed {}ms from target",
            seek_error_ms
        );
    }

    // NOTE: We don't clear the ring buffer here because:
    // 1. Producer doesn't have a clear() method (we're on write side)
    // 2. The decoder will quickly refill with correct samples after seek
    // 3. The consumer might briefly read stale data, but this is acceptable
    //    during a seek operation (position is changing anyway)
    //
    // Update position to ACTUAL position (not requested)
    // This fixes position drift with VBR files where seek lands at a different position
    shared.samples_read.store(
        (actual_position.as_secs_f64() * target_sample_rate as f64 * channels as f64) as usize,
        Ordering::Relaxed,
    );
    shared.is_eof.store(false, Ordering::Relaxed);
    shared.seek_pending.store(false, Ordering::Relaxed);

    // CRITICAL FIX: Increment generation to invalidate stale ring buffer samples
    // This prevents the 3-5 second audio delay by causing the audio callback
    // to skip old samples that were buffered before the seek
    shared.generation.fetch_add(1, Ordering::Release);

    tracing::info!(
        "[DecoderThread] Seek completed: requested={:?}, actual={:?}, generation={}, reset all skip counters",
        position,
        actual_position,
        shared.generation.load(Ordering::Acquire) - 1
    );

    // Return reset skip counters (resampler_skip, encoder_delay_skip)
    Ok((0, 0))
}

impl LocalAudioSource {
    /// Create a new streaming local audio source with background decoder
    ///
    /// Spawns a background thread that handles all decoding and resampling.
    /// The audio callback only reads from a pre-filled buffer, never blocking.
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    /// * `target_sample_rate` - Target output sample rate (e.g., 44100, 48000)
    ///
    /// # Returns
    /// * `Ok(source)` - Audio source ready for streaming playback
    /// * `Err(_)` - Failed to open or probe file
    pub fn new(path: impl AsRef<Path>, target_sample_rate: u32) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Log buffer settings for debugging - this confirms new code is running
        tracing::info!(
            "[LocalAudioSource] INIT: MIN_BUFFER_SAMPLES={} (~{}ms at 48kHz stereo)",
            MIN_BUFFER_SAMPLES,
            MIN_BUFFER_SAMPLES * 1000 / 96000
        );

        // Open the file and probe it to get metadata
        let file = File::open(&path)
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to open file: {}", e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to probe file: {}", e)))?;

        let format_reader = probed.format;

        let track = format_reader
            .default_track()
            .ok_or_else(|| PlaybackError::AudioSource("No audio tracks found".into()))?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;
        let track_id = track.id;
        let time_base = track
            .codec_params
            .time_base
            .unwrap_or(TimeBase::new(1, sample_rate));

        let total_duration = track
            .codec_params
            .n_frames
            .map(|frames| Duration::from_secs_f64(frames as f64 / sample_rate as f64))
            .unwrap_or(Duration::MAX);

        let needs_resampling = sample_rate != target_sample_rate;

        tracing::info!(
            "[LocalAudioSource] Loading file: path={}, source_rate={}Hz, target_rate={}Hz, channels={}, resampling={}",
            path.display(),
            sample_rate,
            target_sample_rate,
            channels,
            needs_resampling
        );

        // Calculate buffer capacity (5 seconds of stereo audio at target sample rate)
        let output_buffer_capacity =
            (BUFFER_SIZE_SECONDS * target_sample_rate as usize) * channels as usize;

        // Create lock-free ring buffer
        let (producer, consumer) = RingBuffer::new(output_buffer_capacity);

        // Create shared state (lock-free atomics)
        let shared = Arc::new(SharedState {
            samples_read: AtomicUsize::new(0),
            is_eof: AtomicBool::new(false),
            seek_pending: AtomicBool::new(false),
            generation: AtomicUsize::new(0),
        });

        // Create command channel
        let (command_tx, command_rx) = bounded::<DecoderCommand>(4);

        // Clone data for decoder thread
        let path_clone = path.clone();
        let shared_clone = shared.clone();

        // Spawn background decoder thread
        let decoder_thread = thread::Builder::new()
            .name(format!(
                "decoder-{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ))
            .spawn(move || {
                Self::decoder_thread_main(
                    path_clone,
                    sample_rate,
                    target_sample_rate,
                    channels,
                    track_id,
                    time_base,
                    producer,
                    shared_clone,
                    command_rx,
                );
            })
            .map_err(|e| {
                PlaybackError::AudioSource(format!("Failed to spawn decoder thread: {}", e))
            })?;

        tracing::debug!("[LocalAudioSource] Background decoder thread started");

        // NOTE: We do NOT wait for buffer to fill here anymore!
        // This was causing 50-200ms delay on playback start.
        //
        // The buffer will fill in the background decoder thread while we return immediately.
        // The TrackLoader and PlaybackManager will check `is_ready()` before starting playback,
        // ensuring smooth, glitch-free playback without blocking the play command.
        //
        // Old behavior (REMOVED):
        // - Block for up to 1 second waiting for 250ms of audio to buffer
        // - Caused 50-200ms delay on every track load (especially with "Maximum" resampling quality)
        //
        // New behavior (CURRENT):
        // - Return immediately after spawning decoder thread
        // - Decoder fills buffer in background (decoding + resampling)
        // - `is_ready()` returns true when MIN_BUFFER_SAMPLES (250ms) is buffered
        // - TrackLoader waits for `is_ready()` AFTER returning the source (non-blocking)
        // - PlaybackManager checks `is_ready()` before starting audio output
        //
        // This eliminates the blocking delay while maintaining buffer safety.

        Ok(Self {
            path,
            source_sample_rate: sample_rate,
            target_sample_rate,
            channels,
            buffer_consumer: consumer,
            shared,
            output_buffer_capacity,
            command_tx,
            _decoder_thread: decoder_thread,
            total_duration,
            needs_resampling,
            last_generation: 0,
        })
    }

    /// Check if this source requires resampling
    ///
    /// Returns true if the source sample rate differs from the target sample rate.
    pub fn needs_resampling(&self) -> bool {
        self.needs_resampling
    }

    /// Background decoder thread main function
    ///
    /// Continuously decodes packets and fills the output buffer.
    /// Handles seek commands and stops when requested.
    fn decoder_thread_main(
        path: PathBuf,
        source_sample_rate: u32,
        target_sample_rate: u32,
        channels: u16,
        track_id: u32,
        time_base: TimeBase,
        mut producer: Producer<f32>,
        shared: Arc<SharedState>,
        command_rx: Receiver<DecoderCommand>,
    ) {
        // Re-open file for this thread (can't send format_reader across threads)
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("[DecoderThread] Failed to open file: {}", e);
                return;
            }
        };

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = match symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("[DecoderThread] Failed to probe file: {}", e);
                return;
            }
        };

        let mut format_reader = probed.format;

        let track = if let Some(t) = format_reader.default_track() {
            t
        } else {
            tracing::error!("[DecoderThread] No audio track found");
            return;
        };

        let mut decoder = match symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
        {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("[DecoderThread] Failed to create decoder: {}", e);
                return;
            }
        };

        // Setup resampler if needed
        let needs_resampling = source_sample_rate != target_sample_rate;
        let resampler_chunk_frames = 1024;

        let mut resampler = if needs_resampling {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let resample_ratio = target_sample_rate as f64 / source_sample_rate as f64;

            match SincFixedIn::<f32>::new(
                resample_ratio,
                2.0,
                params,
                resampler_chunk_frames,
                channels as usize,
            ) {
                Ok(r) => {
                    tracing::debug!(
                        "[DecoderThread] Resampler created (output_delay: {} frames)",
                        r.output_delay()
                    );
                    Some(r)
                }
                Err(e) => {
                    tracing::error!("[DecoderThread] Failed to create resampler: {}", e);
                    return;
                }
            }
        } else {
            None
        };

        // Input buffer for accumulating samples before resampling
        let mut input_buffer: VecDeque<f32> =
            VecDeque::with_capacity(resampler_chunk_frames * channels as usize * 4);
        let mut is_eof = false;

        // Track resampler output delay to skip initial filter ramp-up
        // The sinc resampler produces N output_delay frames of "warming up" audio
        // that contains filter artifacts (amplitude ramp from 0 to full)
        // Skipping these prevents jitter/artifacts at playback start
        let resampler_skip_samples = resampler
            .as_ref()
            .map(|r| {
                let delay_frames = r.output_delay();
                let skip_samples = delay_frames * channels as usize;
                tracing::debug!(
                    "[DecoderThread] Will skip first {} samples ({} frames) for resampler settling",
                    skip_samples,
                    delay_frames
                );
                skip_samples
            })
            .unwrap_or(0);
        let mut resampler_samples_skipped: usize = 0;

        // Track encoder delay to skip codec startup artifacts
        // Get codec-specific encoder delay (FLAC=0, MP3=1152, AAC=1024, etc.)
        let encoder_delay_frames = get_encoder_delay_frames(&track.codec_params);
        let encoder_delay_skip_samples = encoder_delay_frames * channels as usize;
        let mut encoder_delay_samples_skipped: usize = 0;

        tracing::info!(
            "[DecoderThread] Decoder thread ready, codec={:?}, encoder_delay_frames={}, will skip {} encoder delay samples, {} resampler samples",
            track.codec_params.codec,
            encoder_delay_frames,
            encoder_delay_skip_samples,
            resampler_skip_samples
        );

        loop {
            // Check for commands (non-blocking)
            match command_rx.try_recv() {
                Ok(DecoderCommand::Stop) => {
                    tracing::debug!("[DecoderThread] Stop command received, exiting");
                    break;
                }
                Ok(DecoderCommand::Seek(position)) => {
                    tracing::debug!("[DecoderThread] Seek command: {:?}", position);

                    // Use helper function to handle seek
                    match handle_seek_command(
                        &mut format_reader,
                        &mut decoder,
                        &mut resampler,
                        &mut input_buffer,
                        &mut producer,
                        &shared,
                        position,
                        track_id,
                        time_base,
                        target_sample_rate,
                        channels,
                    ) {
                        Ok((resampler_reset, encoder_reset)) => {
                            resampler_samples_skipped = resampler_reset;
                            encoder_delay_samples_skipped = encoder_reset;
                            is_eof = false;
                        }
                        Err(e) => {
                            tracing::error!("[DecoderThread] Seek failed: {}", e);
                        }
                    }
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // No command, continue decoding
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    tracing::debug!("[DecoderThread] Command channel disconnected, exiting");
                    break;
                }
            }

            // Check if buffer needs filling
            let buffer_len = producer.slots();

            // If buffer is full enough, sleep a bit to avoid spinning
            if buffer_len < producer.buffer().capacity() / 2 {
                thread::sleep(Duration::from_millis(10));
                continue;
            }

            // If EOF, nothing more to decode
            if is_eof {
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            // Decode next packet
            let decode_start = std::time::Instant::now();
            let packet = match format_reader.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    is_eof = true;

                    // Flush resampler
                    if needs_resampling && !input_buffer.is_empty() {
                        Self::flush_resampler_static(
                            &mut input_buffer,
                            &mut resampler,
                            channels as usize,
                            resampler_chunk_frames,
                            &mut producer,
                        );
                    }

                    shared.is_eof.store(true, Ordering::Relaxed);
                    tracing::debug!("[DecoderThread] Reached end of file");
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[DecoderThread] Error reading packet"
                    );
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };

            // Skip packets from other tracks
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        decode_time_us = decode_start.elapsed().as_micros(),
                        "[DecoderThread] Decode error"
                    );
                    continue;
                }
            };

            // Convert to f32 samples
            let samples = match Self::convert_to_f32_interleaved(decoded, channels) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "[DecoderThread] Conversion error"
                    );
                    continue;
                }
            };

            let decode_time_us = decode_start.elapsed().as_micros();
            if decode_time_us > 10000 {
                // Log slow decodes (>10ms)
                tracing::warn!(
                    decode_time_us = decode_time_us,
                    samples_decoded = samples.len(),
                    "[DecoderThread] Slow packet decode detected"
                );
            }

            if needs_resampling {
                // Skip encoder delay BEFORE resampling
                // Add samples to input buffer (after encoder delay skip)
                for sample in samples {
                    if encoder_delay_samples_skipped < encoder_delay_skip_samples {
                        encoder_delay_samples_skipped += 1;
                        continue; // Skip this sample
                    }
                    input_buffer.push_back(sample);
                }

                // Log when encoder delay skip completes (resampling path)
                if encoder_delay_samples_skipped >= encoder_delay_skip_samples
                    && encoder_delay_samples_skipped - encoder_delay_skip_samples < 100
                {
                    tracing::debug!(
                        "[DecoderThread] Encoder delay skipping complete (resampling path, skipped {} samples)",
                        encoder_delay_skip_samples
                    );
                }

                // Process resampling (with resampler delay skipping)
                resampler_samples_skipped = Self::process_resampling_with_skip(
                    &mut input_buffer,
                    &mut resampler,
                    channels as usize,
                    resampler_chunk_frames,
                    &mut producer,
                    resampler_skip_samples,
                    resampler_samples_skipped,
                );
            } else {
                // No resampling - add directly to output buffer WITH encoder delay skip
                encoder_delay_samples_skipped = skip_encoder_delay(
                    samples,
                    encoder_delay_skip_samples,
                    encoder_delay_samples_skipped,
                    &mut producer,
                );

                // Log when encoder delay skip completes
                if encoder_delay_samples_skipped >= encoder_delay_skip_samples
                    && encoder_delay_samples_skipped - encoder_delay_skip_samples < 100
                {
                    // Log only once (within 100 samples of completion)
                    tracing::debug!(
                        "[DecoderThread] Encoder delay skipping complete (skipped {} samples)",
                        encoder_delay_skip_samples
                    );
                }
            }
        }

        tracing::debug!("[DecoderThread] Decoder thread exiting");
    }

    /// Process resampling with initial delay skipping
    ///
    /// The sinc resampler produces `output_delay` frames of "warming up" audio
    /// at the start. These frames contain filter artifacts (amplitude ramp from 0)
    /// that can cause audible jitter/clicks at playback start.
    ///
    /// This function skips the first `skip_samples` of output to avoid these artifacts.
    ///
    /// Returns: updated `samples_skipped` counter
    fn process_resampling_with_skip(
        input_buffer: &mut VecDeque<f32>,
        resampler: &mut Option<SincFixedIn<f32>>,
        channels: usize,
        chunk_frames: usize,
        producer: &mut Producer<f32>,
        skip_samples: usize,
        mut samples_skipped: usize,
    ) -> usize {
        let Some(ref mut resampler) = resampler else {
            return samples_skipped;
        };

        let samples_per_chunk = chunk_frames * channels;

        while input_buffer.len() >= samples_per_chunk {
            let mut deinterleaved: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk_frames); channels];

            for _ in 0..chunk_frames {
                for ch in 0..channels {
                    let sample = input_buffer.pop_front().unwrap();
                    deinterleaved[ch].push(sample);
                }
            }

            let resampled = match resampler.process(&deinterleaved, None) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("[DecoderThread] Resampling error: {}", e);
                    return samples_skipped;
                }
            };

            let output_frames = resampled[0].len();
            let output_samples = output_frames * channels;

            // Check if we still need to skip samples (resampler settling period)
            if samples_skipped < skip_samples {
                let samples_to_skip = (skip_samples - samples_skipped).min(output_samples);
                let frames_to_skip = samples_to_skip / channels;
                let start_frame = frames_to_skip;

                if start_frame < output_frames {
                    // Partial skip - add remaining samples after skip
                    for frame_idx in start_frame..output_frames {
                        for ch in 0..channels {
                            // Non-blocking push - if buffer is full, skip the sample
                            let _ = producer.push(resampled[ch][frame_idx]);
                        }
                    }
                }

                samples_skipped += samples_to_skip;

                if samples_skipped >= skip_samples {
                    tracing::debug!(
                        "[DecoderThread] Resampler settling complete, skipped {} samples",
                        samples_skipped
                    );
                }
            } else {
                // Normal operation - add all samples to output buffer
                for frame_idx in 0..output_frames {
                    for ch in 0..channels {
                        // Non-blocking push - if buffer is full, skip the sample
                        let _ = producer.push(resampled[ch][frame_idx]);
                    }
                }
            }
        }

        samples_skipped
    }

    /// Static version of `flush_resampler` for use in decoder thread
    fn flush_resampler_static(
        input_buffer: &mut VecDeque<f32>,
        resampler: &mut Option<SincFixedIn<f32>>,
        channels: usize,
        chunk_frames: usize,
        producer: &mut Producer<f32>,
    ) {
        let Some(ref mut resampler) = resampler else {
            return;
        };

        if input_buffer.is_empty() {
            return;
        }

        let remaining_samples = input_buffer.len();
        let remaining_frames = remaining_samples / channels;

        if remaining_frames == 0 {
            return;
        }

        let mut deinterleaved: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk_frames); channels];

        for _ in 0..remaining_frames {
            for ch in 0..channels {
                let sample = input_buffer.pop_front().unwrap();
                deinterleaved[ch].push(sample);
            }
        }

        let frames_to_pad = chunk_frames - remaining_frames;
        for ch in 0..channels {
            deinterleaved[ch].extend(std::iter::repeat_n(0.0f32, frames_to_pad));
        }

        let resampled = match resampler.process(&deinterleaved, None) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("[DecoderThread] Flush resampling error: {}", e);
                return;
            }
        };

        let output_frames = resampled[0].len();
        let valid_output_frames =
            (remaining_frames as f64 / chunk_frames as f64 * output_frames as f64) as usize;

        for frame_idx in 0..valid_output_frames {
            for ch in 0..channels {
                // Non-blocking push - if buffer is full, skip the sample
                let _ = producer.push(resampled[ch][frame_idx]);
            }
        }
    }

    /// Generic helper to interleave planar audio buffer to stereo f32
    ///
    /// Takes any planar buffer type and a normalization function,
    /// converts to interleaved stereo f32 format with mono->stereo duplication.
    ///
    /// # Type Parameters
    /// * `T` - Sample type (i8, i16, i32, u8, u16, u32, f32, f64, etc.)
    /// * `F` - Normalization function: T -> f32 in range [-1.0, 1.0]
    fn interleave_to_stereo_f32<T, F>(
        buf: &symphonia::core::audio::AudioBuffer<T>,
        normalize: F,
    ) -> Vec<f32>
    where
        T: symphonia::core::sample::Sample,
        F: Fn(T) -> f32,
    {
        let channels = buf.spec().channels.count();
        let frames = buf.frames();
        let mut output = Vec::with_capacity(frames * 2);

        for frame_idx in 0..frames {
            // Left channel
            output.push(normalize(buf.chan(0)[frame_idx]));
            // Right channel (duplicate left if mono)
            if channels > 1 {
                output.push(normalize(buf.chan(1)[frame_idx]));
            } else {
                output.push(normalize(buf.chan(0)[frame_idx]));
            }
        }

        output
    }

    /// Convert Symphonia `AudioBufferRef` to interleaved f32 samples
    ///
    /// Handles all Symphonia sample formats:
    /// - Float: F32, F64
    /// - Signed int: S8, S16, S24, S32
    /// - Unsigned int: U8, U16, U24, U32
    ///
    /// All formats are normalized to [-1.0, 1.0] and converted to stereo.
    fn convert_to_f32_interleaved(
        decoded: AudioBufferRef,
        _target_channels: u16,
    ) -> Result<Vec<f32>> {
        let output = match decoded {
            // Float formats - clamp to [-1.0, 1.0] to handle intersample peaks
            AudioBufferRef::F32(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| s.clamp(-1.0, 1.0))
            }
            AudioBufferRef::F64(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| (s as f32).clamp(-1.0, 1.0))
            }

            // Signed integer formats - use symmetric scaling (divide by 2^(N-1))
            // This ensures -1.0 to 1.0 range is symmetric
            AudioBufferRef::S8(buf) => Self::interleave_to_stereo_f32(&buf, |s| s as f32 / 128.0),
            AudioBufferRef::S16(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| s as f32 / 32768.0)
            }
            AudioBufferRef::S24(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| s.inner() as f32 / 8388608.0)
            }
            AudioBufferRef::S32(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| s as f32 / 2147483648.0)
            }

            // Unsigned integer formats - normalize and center around 0
            AudioBufferRef::U8(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0)
            }
            AudioBufferRef::U16(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
            }
            AudioBufferRef::U24(buf) => {
                // U24 range: 0 to 16777215 (2^24 - 1)
                Self::interleave_to_stereo_f32(&buf, |s| {
                    (s.inner() as f32 / 16777215.0) * 2.0 - 1.0
                })
            }
            AudioBufferRef::U32(buf) => {
                Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0)
            }
        };

        Ok(output)
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get target sample rate (output sample rate)
    pub fn sample_rate(&self) -> u32 {
        self.target_sample_rate
    }

    /// Get source sample rate (file's original sample rate)
    pub fn source_sample_rate(&self) -> u32 {
        self.source_sample_rate
    }

    /// Get number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

impl AudioSource for LocalAudioSource {
    /// Read samples from the pre-filled buffer (non-blocking, wait-free)
    ///
    /// This method NEVER blocks on disk I/O - it only reads from the lock-free buffer
    /// that the background decoder thread has filled. If the buffer is empty,
    /// it fills with silence to prevent glitches.
    ///
    /// Uses rtrb ring buffer for lock-free, wait-free reads from the audio thread.
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize> {
        // CRITICAL FIX: Check if generation changed (seek occurred)
        // If so, drain all stale samples from ring buffer to prevent 3-5s audio delay
        let current_generation = self.shared.generation.load(Ordering::Acquire);
        if current_generation != self.last_generation {
            // Seek detected! Drain ring buffer of stale samples
            let mut drained = 0usize;
            while let Ok(chunk) = self.buffer_consumer.read_chunk(self.output_buffer_capacity) {
                if chunk.is_empty() {
                    break;
                }
                drained += chunk.len();
                chunk.commit_all();
            }
            self.last_generation = current_generation;
            tracing::info!(
                "[SEEK PERF] Drained {} stale samples from ring buffer (generation {} -> {})",
                drained,
                current_generation - 1,
                current_generation
            );
        }

        // Wait-free read from ring buffer using read_chunk for efficiency
        let available = self.buffer_consumer.read_chunk(output.len());
        let read_count = if let Ok(chunk) = available {
            let count = chunk.len();
            output[..count].copy_from_slice(chunk.as_slices().0);
            // If the chunk wraps around the ring buffer, copy the second part
            if !chunk.as_slices().1.is_empty() {
                let first_len = chunk.as_slices().0.len();
                output[first_len..count].copy_from_slice(chunk.as_slices().1);
            }
            chunk.commit_all();
            count
        } else {
            0
        };

        // Track first read and underruns for logging
        let samples_read = self.shared.samples_read.load(Ordering::Relaxed);
        let is_first_read = samples_read == 0;
        let is_eof = self.shared.is_eof.load(Ordering::Relaxed);
        let is_underrun = read_count < output.len() && !is_eof;

        // Update samples read counter
        self.shared
            .samples_read
            .fetch_add(read_count, Ordering::Relaxed);

        // Log diagnostics using trace! to minimize overhead in hot path
        if is_first_read {
            tracing::trace!(
                "[LocalAudioSource::read_samples] First read: requested={}, available={}",
                output.len(),
                read_count
            );
        } else if is_underrun {
            // Underruns are logged at debug level since they indicate potential issues
            tracing::debug!(
                "[LocalAudioSource::read_samples] Buffer underrun: requested={}, available={}",
                output.len(),
                read_count
            );
        }

        // Fill remainder with silence if buffer is empty
        if read_count < output.len() {
            output[read_count..].fill(0.0);
        }

        Ok(read_count)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.total_duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }

        // Mark seek as pending
        self.shared.seek_pending.store(true, Ordering::Relaxed);

        // Send seek command to decoder thread
        self.command_tx
            .send(DecoderCommand::Seek(position))
            .map_err(|e| {
                PlaybackError::AudioSource(format!("Failed to send seek command: {}", e))
            })?;

        Ok(())
    }

    fn duration(&self) -> Duration {
        self.total_duration
    }

    fn position(&self) -> Duration {
        // Calculate position based on samples read (at target sample rate)
        // Lock-free read using atomic
        let samples_read = self.shared.samples_read.load(Ordering::Relaxed);
        let frames = samples_read / self.channels as usize;
        Duration::from_secs_f64(frames as f64 / self.target_sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        // Lock-free check using atomics and ring buffer
        self.shared.is_eof.load(Ordering::Relaxed) && self.buffer_consumer.is_empty()
    }

    /// Check if source is ready for glitch-free playback
    ///
    /// Returns true when:
    /// - Buffer contains at least `MIN_BUFFER_SAMPLES` (250ms of audio at 48kHz stereo)
    /// - OR we've reached EOF (short files)
    ///
    /// This prevents buffer underrun at playback start when disk I/O is slow.
    /// Lock-free check using ring buffer slots count.
    fn is_ready(&self) -> bool {
        // Ready if we have enough samples OR if we've reached EOF (short files)
        let buffer_len = self.buffer_consumer.slots();
        buffer_len >= MIN_BUFFER_SAMPLES || self.shared.is_eof.load(Ordering::Relaxed)
    }

    /// Get sample rate of the audio source (target/output rate)
    ///
    /// Returns the target sample rate after resampling, not the source file's rate.
    fn sample_rate(&self) -> Option<u32> {
        Some(self.target_sample_rate)
    }
}

impl Drop for LocalAudioSource {
    fn drop(&mut self) {
        // Signal decoder thread to stop
        let _ = self.command_tx.send(DecoderCommand::Stop);

        // Note: We intentionally don't join the thread here because:
        // 1. The thread will exit when it receives the Stop command or when the channel disconnects
        // 2. Joining could block the audio thread if the decoder is stuck on I/O
        // 3. The JoinHandle being dropped will detach the thread, allowing it to clean up asynchronously
        //
        // The thread is designed to exit quickly when Stop is received (it checks for commands
        // on each loop iteration), so detaching is safe here.
        //
        // If deterministic cleanup is needed in the future, consider:
        // - Moving thread join to a background cleanup task
        // - Using a timeout with thread::JoinHandle::join
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn local_source_implements_audio_source() {
        // This test ensures the trait is implemented
        fn assert_audio_source<T: AudioSource>() {}
        assert_audio_source::<LocalAudioSource>();
    }

    /// Helper to get test audio file paths
    fn get_test_file(filename: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // libraries
        path.pop(); // root
        path.push("applications/marketing/public/demo-audio");
        path.push(filename);
        path
    }

    #[test]
    fn test_mp3_format_loading() {
        let path = get_test_file("dark.mp3");
        if !path.exists() {
            println!("Skipping test - demo file not found: {:?}", path);
            return;
        }

        let source = LocalAudioSource::new(&path, 44100);
        assert!(
            source.is_ok(),
            "Failed to load MP3 file: {:?}",
            source.err()
        );

        let source = source.unwrap();
        assert_eq!(source.channels(), 2, "Expected stereo audio");
        assert!(source.sample_rate() > 0, "Sample rate should be positive");
        assert!(
            source.duration() > Duration::from_secs(0),
            "Duration should be positive"
        );
    }

    #[test]
    fn test_flac_format_loading() {
        let path = get_test_file("dark.flac");
        if !path.exists() {
            println!("Skipping test - demo file not found: {:?}", path);
            return;
        }

        let source = LocalAudioSource::new(&path, 44100);
        assert!(
            source.is_ok(),
            "Failed to load FLAC file: {:?}",
            source.err()
        );

        let source = source.unwrap();
        assert_eq!(source.channels(), 2, "Expected stereo audio");
        assert!(source.sample_rate() > 0, "Sample rate should be positive");
        assert!(
            source.duration() > Duration::from_secs(0),
            "Duration should be positive"
        );
    }

    #[test]
    fn test_format_consistency() {
        // Both MP3 and FLAC versions of the same track should have similar properties
        let mp3_path = get_test_file("dark.mp3");
        let flac_path = get_test_file("dark.flac");

        if !mp3_path.exists() || !flac_path.exists() {
            println!("Skipping test - demo files not found");
            return;
        }

        let mp3_source = LocalAudioSource::new(&mp3_path, 44100).expect("Failed to load MP3");
        let flac_source = LocalAudioSource::new(&flac_path, 44100).expect("Failed to load FLAC");

        // Both should have same channel count
        assert_eq!(mp3_source.channels(), flac_source.channels());

        // Sample rates should match target (44100)
        assert_eq!(mp3_source.sample_rate(), 44100);
        assert_eq!(flac_source.sample_rate(), 44100);
    }

    #[test]
    fn test_read_samples() {
        let path = get_test_file("dark.mp3");
        if !path.exists() {
            println!("Skipping test - demo file not found");
            return;
        }

        let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");
        let mut buffer = vec![0.0f32; 1024];

        // Wait for background decoder thread to fill buffer (max 1 second)
        let start = std::time::Instant::now();
        let mut samples_read = 0;
        while samples_read == 0 && start.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(10));
            let read = source.read_samples(&mut buffer);
            assert!(read.is_ok(), "Failed to read samples: {:?}", read.err());
            samples_read = read.unwrap();
        }

        assert!(samples_read > 0, "Should read at least some samples");
        assert!(
            samples_read <= buffer.len(),
            "Shouldn't read more than buffer size"
        );

        // Verify samples are in valid range [-1.0, 1.0]
        for (i, &sample) in buffer.iter().enumerate().take(samples_read) {
            assert!(
                (-1.0..=1.0).contains(&sample),
                "Sample {} at index {} is out of range [-1.0, 1.0]",
                sample,
                i
            );
        }
    }

    #[test]
    fn test_position_tracking() {
        let path = get_test_file("dark.mp3");
        if !path.exists() {
            println!("Skipping test - demo file not found");
            return;
        }

        let mut source = LocalAudioSource::new(&path, 44100).expect("Failed to load MP3");

        // Initial position should be 0
        assert_eq!(source.position(), Duration::from_secs(0));

        // Wait for background decoder to fill buffer and read samples (max 1 second)
        let mut buffer = vec![0.0f32; 4410]; // ~0.05 seconds at 44.1kHz stereo
        let start = std::time::Instant::now();
        let mut samples_read = 0;
        while samples_read == 0 && start.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(10));
            samples_read = source.read_samples(&mut buffer).unwrap_or(0);
        }

        assert!(samples_read > 0, "Should have read samples after waiting");

        // Position should have advanced
        assert!(
            source.position() > Duration::from_secs(0),
            "Position should advance after reading"
        );
        assert!(
            source.position() < source.duration(),
            "Position shouldn't exceed duration"
        );
    }

    #[test]
    fn test_sample_rate_conversion() {
        let path = get_test_file("dark.mp3");
        if !path.exists() {
            println!("Skipping test - demo file not found");
            return;
        }

        // Test different target sample rates
        let rates = vec![44100, 48000, 22050];

        for target_rate in rates {
            let source = LocalAudioSource::new(&path, target_rate)
                .expect("Failed to create source with target rate");

            // Verify output sample rate matches target
            assert_eq!(
                source.sample_rate(),
                target_rate,
                "Output sample rate should match target"
            );

            // Source rate might be different
            println!(
                "Source rate: {}, Target rate: {}, Needs resampling: {}",
                source.source_sample_rate(),
                source.sample_rate(),
                source.needs_resampling
            );
        }
    }

    #[test]
    fn test_playback_speed_with_resampling() {
        let path = get_test_file("dark.mp3");
        if !path.exists() {
            println!("Skipping test - demo file not found");
            return;
        }

        // Create sources with different target sample rates
        let mut source_44100 =
            LocalAudioSource::new(&path, 44100).expect("Failed to load at 44.1kHz");
        let mut source_48000 =
            LocalAudioSource::new(&path, 48000).expect("Failed to load at 48kHz");

        // Both should report the same duration (in seconds)
        let duration_44100 = source_44100.duration();
        let duration_48000 = source_48000.duration();

        assert!(
            (duration_44100.as_secs_f64() - duration_48000.as_secs_f64()).abs() < 0.1,
            "Duration should be consistent regardless of sample rate conversion (got {:.2}s vs {:.2}s)",
            duration_44100.as_secs_f64(),
            duration_48000.as_secs_f64()
        );

        // Read samples in chunks (background decoder is async)
        // We'll read ~0.5 second worth of samples in chunks
        let chunk_size = 4096;
        let target_samples_44100 = 44100; // 0.5 second stereo at 44.1kHz
        let target_samples_48000 = 48000; // 0.5 second stereo at 48kHz

        let mut total_read_44100 = 0;
        let mut total_read_48000 = 0;
        let mut buffer = vec![0.0f32; chunk_size];

        // Read from 44.1kHz source in chunks, waiting for data
        for _ in 0..100 {
            let read = source_44100.read_samples(&mut buffer).expect("Read failed");
            total_read_44100 += read;
            if total_read_44100 >= target_samples_44100 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Read from 48kHz source in chunks, waiting for data
        for _ in 0..100 {
            let read = source_48000.read_samples(&mut buffer).expect("Read failed");
            total_read_48000 += read;
            if total_read_48000 >= target_samples_48000 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Should have read enough samples
        assert!(
            total_read_44100 >= target_samples_44100,
            "Should read at least 0.5 second at 44.1kHz (got {} samples)",
            total_read_44100
        );
        assert!(
            total_read_48000 >= target_samples_48000,
            "Should read at least 0.5 second at 48kHz (got {} samples)",
            total_read_48000
        );

        // Position should have advanced
        let pos_44100 = source_44100.position().as_secs_f64();
        let pos_48000 = source_48000.position().as_secs_f64();

        assert!(
            pos_44100 > 0.4,
            "Position should be at least ~0.4s at 44.1kHz (got {:.2}s)",
            pos_44100
        );
        assert!(
            pos_48000 > 0.4,
            "Position should be at least ~0.4s at 48kHz (got {:.2}s)",
            pos_48000
        );
    }
}
