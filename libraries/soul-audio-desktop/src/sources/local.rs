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
//!    - Continuously decodes packets and fills output_buffer
//!    - Handles all disk I/O and resampling off the audio thread
//!    - Keeps buffer at ~5 seconds of audio
//!
//! 2. **Non-blocking read_samples()**:
//!    - Only reads from output_buffer (no decoding)
//!    - Returns available samples or silence if buffer empty
//!    - Never blocks on disk I/O
//!
//! 3. **Format Conversion** (`convert_to_f32_interleaved`):
//!    - Handles all Symphonia sample formats
//!    - Normalizes to [-1.0, 1.0] range

use crossbeam_channel::{bounded, Receiver, Sender};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use soul_playback::{AudioSource, PlaybackError, Result};
use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;

/// Size of ring buffer in seconds
const BUFFER_SIZE_SECONDS: usize = 5;

/// Minimum buffer level (in samples) before source is considered ready for playback
/// At stereo 48kHz, 48000 samples = 500ms of audio (foobar2000 uses 1000ms default)
///
/// This MUST be filled BEFORE playback starts to prevent buffer underrun.
/// The user reported jitter on first play but not on "previous track" because:
/// - First play: file not cached → slow disk I/O → buffer underrun
/// - Previous track: file cached → fast I/O → no underrun
///
/// Industry standard: prebuffer 500ms-1000ms before playback starts
const MIN_BUFFER_SAMPLES: usize = 48000;

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
    /// Ring buffer for resampled samples (at TARGET rate, ready for output)
    output_buffer: VecDeque<f32>,
    /// Total samples read by audio callback (at target rate)
    samples_read: usize,
    /// Whether decoder has reached end of file
    is_eof: bool,
    /// Whether a seek is pending (decoder will reset)
    seek_pending: bool,
}

/// Audio source for local files with background decoder thread
///
/// Uses Symphonia to decode audio files from disk in a background thread.
/// The audio callback only reads from a pre-filled buffer, never blocking on I/O.
/// Maintains a 5-second ring buffer for smooth, glitch-free playback.
/// Automatically resamples audio to match target sample rate.
///
/// Supports all formats: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct LocalAudioSource {
    path: PathBuf,
    source_sample_rate: u32, // Sample rate of the audio file
    target_sample_rate: u32, // Target output sample rate
    channels: u16,

    // Shared state with decoder thread (protected by mutex)
    shared: Arc<Mutex<SharedState>>,
    output_buffer_capacity: usize, // Max samples to buffer

    // Communication with decoder thread
    command_tx: Sender<DecoderCommand>,
    _decoder_thread: JoinHandle<()>,

    // Position tracking
    total_duration: Duration,
    needs_resampling: bool,
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

        eprintln!("[LocalAudioSource] File info:");
        eprintln!("  - Path: {}", path.display());
        eprintln!("  - Source sample rate: {} Hz", sample_rate);
        eprintln!("  - Target sample rate: {} Hz", target_sample_rate);
        eprintln!("  - Channels: {}", channels);
        eprintln!("  - Needs resampling: {}", needs_resampling);

        // Calculate buffer capacity (5 seconds of stereo audio at target sample rate)
        let output_buffer_capacity =
            (BUFFER_SIZE_SECONDS * target_sample_rate as usize) * channels as usize;

        // Create shared state
        let shared = Arc::new(Mutex::new(SharedState {
            output_buffer: VecDeque::with_capacity(output_buffer_capacity),
            samples_read: 0,
            is_eof: false,
            seek_pending: false,
        }));

        // Create command channel
        let (command_tx, command_rx) = bounded::<DecoderCommand>(4);

        // Clone data for decoder thread
        let path_clone = path.clone();
        let shared_clone = shared.clone();

        // Spawn background decoder thread
        let decoder_thread = thread::Builder::new()
            .name(format!("decoder-{}", path.file_name().unwrap_or_default().to_string_lossy()))
            .spawn(move || {
                Self::decoder_thread_main(
                    path_clone,
                    sample_rate,
                    target_sample_rate,
                    channels,
                    track_id,
                    time_base,
                    output_buffer_capacity,
                    shared_clone,
                    command_rx,
                );
            })
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to spawn decoder thread: {}", e)))?;

        eprintln!("[LocalAudioSource] Background decoder thread started");

        // Wait for buffer to have sufficient data (up to 1 second)
        // This ensures the source is immediately playable without underrun
        // 200ms buffer (MIN_BUFFER_SAMPLES) prevents jitter on first play
        let min_initial_samples = MIN_BUFFER_SAMPLES;
        let mut buffer_filled = false;
        for _ in 0..100 {
            let buffer_len = {
                let state = shared.lock().unwrap();
                state.output_buffer.len()
            };
            if buffer_len >= min_initial_samples {
                eprintln!("[LocalAudioSource] Initial buffer filled: {} samples (~{}ms)",
                    buffer_len,
                    buffer_len * 1000 / (target_sample_rate as usize * channels as usize)
                );
                buffer_filled = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        if !buffer_filled {
            let buffer_len = shared.lock().unwrap().output_buffer.len();
            eprintln!(
                "[LocalAudioSource] WARNING: Initial buffer not fully filled ({} samples, wanted {}). May cause jitter.",
                buffer_len, min_initial_samples
            );
        }

        Ok(Self {
            path,
            source_sample_rate: sample_rate,
            target_sample_rate,
            channels,
            shared,
            output_buffer_capacity,
            command_tx,
            _decoder_thread: decoder_thread,
            total_duration,
            needs_resampling,
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
        output_buffer_capacity: usize,
        shared: Arc<Mutex<SharedState>>,
        command_rx: Receiver<DecoderCommand>,
    ) {
        // Re-open file for this thread (can't send format_reader across threads)
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[DecoderThread] Failed to open file: {}", e);
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
                eprintln!("[DecoderThread] Failed to probe file: {}", e);
                return;
            }
        };

        let mut format_reader = probed.format;

        let track = match format_reader.default_track() {
            Some(t) => t,
            None => {
                eprintln!("[DecoderThread] No audio track found");
                return;
            }
        };

        let mut decoder = match symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
        {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[DecoderThread] Failed to create decoder: {}", e);
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
                    eprintln!(
                        "[DecoderThread] Resampler created (output_delay: {} frames)",
                        r.output_delay()
                    );
                    Some(r)
                }
                Err(e) => {
                    eprintln!("[DecoderThread] Failed to create resampler: {}", e);
                    return;
                }
            }
        } else {
            None
        };

        // Input buffer for accumulating samples before resampling
        let mut input_buffer: VecDeque<f32> = VecDeque::with_capacity(resampler_chunk_frames * channels as usize * 4);
        let mut is_eof = false;

        // Track resampler output delay to skip initial filter ramp-up
        // The sinc resampler produces N output_delay frames of "warming up" audio
        // that contains filter artifacts (amplitude ramp from 0 to full)
        // Skipping these prevents jitter/artifacts at playback start
        let resampler_skip_samples = resampler.as_ref().map(|r| {
            let delay_frames = r.output_delay();
            let skip_samples = delay_frames * channels as usize;
            eprintln!("[DecoderThread] Will skip first {} samples ({} frames) for resampler settling",
                skip_samples, delay_frames);
            skip_samples
        }).unwrap_or(0);
        let mut resampler_samples_skipped: usize = 0;

        eprintln!("[DecoderThread] Decoder thread ready, starting decode loop");

        loop {
            // Check for commands (non-blocking)
            match command_rx.try_recv() {
                Ok(DecoderCommand::Stop) => {
                    eprintln!("[DecoderThread] Stop command received, exiting");
                    break;
                }
                Ok(DecoderCommand::Seek(position)) => {
                    eprintln!("[DecoderThread] Seek command: {:?}", position);

                    // Perform seek
                    let seek_ts = time_base.calc_timestamp(position.into());
                    if let Err(e) = format_reader.seek(
                        symphonia::core::formats::SeekMode::Accurate,
                        symphonia::core::formats::SeekTo::TimeStamp {
                            ts: seek_ts,
                            track_id,
                        },
                    ) {
                        eprintln!("[DecoderThread] Seek failed: {}", e);
                    } else {
                        decoder.reset();
                        input_buffer.clear();
                        if let Some(ref mut r) = resampler {
                            r.reset();
                        }
                        is_eof = false;

                        // Reset resampler skip counter - need to skip delay samples again after seek
                        resampler_samples_skipped = 0;

                        // Clear output buffer and update position
                        let mut state = shared.lock().unwrap();
                        state.output_buffer.clear();
                        state.samples_read = (position.as_secs_f64()
                            * target_sample_rate as f64
                            * channels as f64) as usize;
                        state.is_eof = false;
                        state.seek_pending = false;
                    }
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // No command, continue decoding
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    eprintln!("[DecoderThread] Command channel disconnected, exiting");
                    break;
                }
            }

            // Check if buffer needs filling
            let buffer_len = {
                let state = shared.lock().unwrap();
                state.output_buffer.len()
            };

            // If buffer is full enough, sleep a bit to avoid spinning
            if buffer_len >= output_buffer_capacity / 2 {
                thread::sleep(Duration::from_millis(10));
                continue;
            }

            // If EOF, nothing more to decode
            if is_eof {
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            // Decode next packet
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
                            &shared,
                        );
                    }

                    let mut state = shared.lock().unwrap();
                    state.is_eof = true;
                    eprintln!("[DecoderThread] Reached end of file");
                    continue;
                }
                Err(e) => {
                    eprintln!("[DecoderThread] Error reading packet: {}", e);
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
                    eprintln!("[DecoderThread] Decode error: {}", e);
                    continue;
                }
            };

            // Convert to f32 samples
            let samples = match Self::convert_to_f32_interleaved(decoded, channels) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[DecoderThread] Conversion error: {}", e);
                    continue;
                }
            };

            if needs_resampling {
                // Add samples to input buffer
                for sample in samples {
                    input_buffer.push_back(sample);
                }

                // Process resampling (with initial delay skipping)
                resampler_samples_skipped = Self::process_resampling_with_skip(
                    &mut input_buffer,
                    &mut resampler,
                    channels as usize,
                    resampler_chunk_frames,
                    output_buffer_capacity,
                    &shared,
                    resampler_skip_samples,
                    resampler_samples_skipped,
                );
            } else {
                // No resampling - add directly to output buffer
                let mut state = shared.lock().unwrap();
                for sample in samples {
                    state.output_buffer.push_back(sample);
                    if state.output_buffer.len() > output_buffer_capacity {
                        state.output_buffer.pop_front();
                    }
                }
            }
        }

        eprintln!("[DecoderThread] Decoder thread exiting");
    }

    /// Process resampling with initial delay skipping
    ///
    /// The sinc resampler produces `output_delay` frames of "warming up" audio
    /// at the start. These frames contain filter artifacts (amplitude ramp from 0)
    /// that can cause audible jitter/clicks at playback start.
    ///
    /// This function skips the first `skip_samples` of output to avoid these artifacts.
    ///
    /// Returns: updated samples_skipped counter
    fn process_resampling_with_skip(
        input_buffer: &mut VecDeque<f32>,
        resampler: &mut Option<SincFixedIn<f32>>,
        channels: usize,
        chunk_frames: usize,
        output_buffer_capacity: usize,
        shared: &Arc<Mutex<SharedState>>,
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
                    eprintln!("[DecoderThread] Resampling error: {}", e);
                    return samples_skipped;
                }
            };

            let output_frames = resampled[0].len();
            let output_samples = output_frames * channels;
            let mut state = shared.lock().unwrap();

            // Check if we still need to skip samples (resampler settling period)
            if samples_skipped < skip_samples {
                let samples_to_skip = (skip_samples - samples_skipped).min(output_samples);
                let frames_to_skip = samples_to_skip / channels;
                let start_frame = frames_to_skip;

                if start_frame < output_frames {
                    // Partial skip - add remaining samples after skip
                    for frame_idx in start_frame..output_frames {
                        for ch in 0..channels {
                            state.output_buffer.push_back(resampled[ch][frame_idx]);
                            if state.output_buffer.len() > output_buffer_capacity {
                                state.output_buffer.pop_front();
                            }
                        }
                    }
                }

                samples_skipped += samples_to_skip;

                if samples_skipped >= skip_samples {
                    eprintln!("[DecoderThread] Resampler settling complete, skipped {} samples", samples_skipped);
                }
            } else {
                // Normal operation - add all samples to output buffer
                for frame_idx in 0..output_frames {
                    for ch in 0..channels {
                        state.output_buffer.push_back(resampled[ch][frame_idx]);
                        if state.output_buffer.len() > output_buffer_capacity {
                            state.output_buffer.pop_front();
                        }
                    }
                }
            }
        }

        samples_skipped
    }

    /// Static version of flush_resampler for use in decoder thread
    fn flush_resampler_static(
        input_buffer: &mut VecDeque<f32>,
        resampler: &mut Option<SincFixedIn<f32>>,
        channels: usize,
        chunk_frames: usize,
        shared: &Arc<Mutex<SharedState>>,
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
            deinterleaved[ch].extend(std::iter::repeat(0.0f32).take(frames_to_pad));
        }

        let resampled = match resampler.process(&deinterleaved, None) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[DecoderThread] Flush resampling error: {}", e);
                return;
            }
        };

        let output_frames = resampled[0].len();
        let valid_output_frames =
            (remaining_frames as f64 / chunk_frames as f64 * output_frames as f64) as usize;

        let mut state = shared.lock().unwrap();
        for frame_idx in 0..valid_output_frames {
            for ch in 0..channels {
                state.output_buffer.push_back(resampled[ch][frame_idx]);
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
    /// Read samples from the pre-filled buffer (non-blocking)
    ///
    /// This method NEVER blocks on disk I/O - it only reads from the buffer
    /// that the background decoder thread has filled. If the buffer is empty,
    /// it fills with silence to prevent glitches.
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize> {
        let mut state = self.shared.lock().unwrap();

        // Copy from output buffer to output (non-blocking)
        let available = state.output_buffer.len().min(output.len());

        // Log first read and any underruns
        let is_first_read = state.samples_read == 0;
        let is_underrun = available < output.len();

        if is_first_read || is_underrun {
            eprintln!(
                "[LocalAudioSource::read_samples] {} requested={}, available={}, buffer_remaining={}, samples_read={}",
                if is_first_read { "FIRST READ:" } else { "UNDERRUN:" },
                output.len(),
                available,
                state.output_buffer.len(),
                state.samples_read
            );
        }

        for i in 0..available {
            output[i] = state.output_buffer.pop_front().unwrap();
        }

        // Log first samples on first read
        if is_first_read && available >= 8 {
            eprintln!(
                "[LocalAudioSource::read_samples] First 8 samples: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                output[0], output[1], output[2], output[3], output[4], output[5], output[6], output[7]
            );
        }

        state.samples_read += available;

        // Fill remainder with silence if buffer is empty
        if available < output.len() {
            output[available..].fill(0.0);
        }

        Ok(available)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.total_duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }

        // Mark seek as pending
        {
            let mut state = self.shared.lock().unwrap();
            state.seek_pending = true;
        }

        // Send seek command to decoder thread
        self.command_tx
            .send(DecoderCommand::Seek(position))
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to send seek command: {}", e)))?;

        Ok(())
    }

    fn duration(&self) -> Duration {
        self.total_duration
    }

    fn position(&self) -> Duration {
        // Calculate position based on samples read (at target sample rate)
        let state = self.shared.lock().unwrap();
        let frames = state.samples_read / self.channels as usize;
        Duration::from_secs_f64(frames as f64 / self.target_sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        let state = self.shared.lock().unwrap();
        state.is_eof && state.output_buffer.is_empty()
    }

    /// Check if source is ready for glitch-free playback
    ///
    /// Returns true when:
    /// - Buffer contains at least MIN_BUFFER_SAMPLES (500ms of audio)
    /// - OR we've reached EOF (short files)
    ///
    /// This prevents buffer underrun at playback start when disk I/O is slow.
    fn is_ready(&self) -> bool {
        let state = self.shared.lock().unwrap();
        // Ready if we have enough samples OR if we've reached EOF (short files)
        state.output_buffer.len() >= MIN_BUFFER_SAMPLES || state.is_eof
    }
}

impl Drop for LocalAudioSource {
    fn drop(&mut self) {
        // Signal decoder thread to stop
        let _ = self.command_tx.send(DecoderCommand::Stop);
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

        // Should be able to read samples
        let read = source.read_samples(&mut buffer);
        assert!(read.is_ok(), "Failed to read samples: {:?}", read.err());

        let samples_read = read.unwrap();
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

        // Read some samples
        let mut buffer = vec![0.0f32; 4410]; // ~0.05 seconds at 44.1kHz stereo
        let _ = source.read_samples(&mut buffer);

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
