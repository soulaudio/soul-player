/// Audio decoder implementation using Symphonia
use crate::error::{AudioError, Result};
use crate::metadata::{self, AudioMetadata as FileMetadata};
use soul_core::{
    AudioBuffer, AudioDecoder as AudioDecoderTrait, AudioFormat, AudioMetadata, SampleRate,
};
use std::path::Path;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Time, TimeBase};

/// Audio decoder using Symphonia
///
/// Supports: MP3, FLAC, OGG, WAV, AAC, OPUS
///
/// This decoder supports two modes:
/// 1. **Full decode**: Use `decode()` to load entire file into memory
/// 2. **Streaming decode**: Use `open()`, `decode_chunk()`, `seek()` for streaming playback
pub struct SymphoniaDecoder {
    /// Streaming state (when a file is open for streaming)
    stream_state: Option<StreamState>,
}

/// Internal state for streaming decode
struct StreamState {
    /// Format reader (container parser)
    format: Box<dyn FormatReader>,
    /// Audio decoder
    decoder: Box<dyn Decoder>,
    /// Track ID
    track_id: u32,
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u16,
    /// Total duration
    duration: Option<Duration>,
    /// Current position in samples
    position_samples: u64,
    /// Time base for position calculation
    time_base: Option<TimeBase>,
}

impl SymphoniaDecoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Self { stream_state: None }
    }

    /// Open a file and create stream state
    fn create_stream_state(path: &Path) -> Result<StreamState> {
        tracing::debug!(file_path = %path.display(), "[Decoder] Opening audio file");
        let start = std::time::Instant::now();

        // Check if file exists
        if !path.exists() {
            tracing::error!(file_path = %path.display(), "[Decoder] File not found");
            return Err(AudioError::FileNotFound(path.display().to_string()));
        }

        // Open the file
        let file = std::fs::File::open(path).map_err(|e| {
            tracing::error!(file_path = %path.display(), error = %e, "[Decoder] Failed to open file");
            AudioError::Io(e)
        })?;

        // Create media source
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        // Probe the media source
        tracing::debug!(file_path = %path.display(), "[Decoder] Probing audio format");
        let probe_start = std::time::Instant::now();
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| {
                tracing::error!(file_path = %path.display(), error = %e, "[Decoder] Failed to probe file format");
                AudioError::Symphonia(format!("Failed to probe file: {}", e))
            })?;
        let probe_duration = probe_start.elapsed();
        tracing::debug!(
            duration_ms = probe_duration.as_millis(),
            "[Decoder] Format probe completed"
        );

        let format = probed.format;

        // Find the default track
        let track = format
            .default_track()
            .ok_or_else(|| AudioError::DecodeError("No audio tracks found".to_string()))?;

        // Get codec parameters
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let track_id = track.id;
        let time_base = track.codec_params.time_base;

        // Calculate duration if possible
        let duration = track
            .codec_params
            .n_frames
            .map(|n_frames| Duration::from_secs_f64(n_frames as f64 / sample_rate as f64));

        // Create decoder
        tracing::debug!(
            sample_rate,
            channels,
            codec = ?track.codec_params.codec,
            "[Decoder] Creating audio decoder"
        );
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| {
                tracing::error!(error = %e, "[Decoder] Failed to create decoder");
                AudioError::Symphonia(format!("Failed to create decoder: {}", e))
            })?;

        let total_duration = start.elapsed();
        tracing::info!(
            file_path = %path.display(),
            sample_rate,
            channels,
            duration_secs = ?duration,
            codec = ?track.codec_params.codec,
            total_duration_ms = total_duration.as_millis(),
            "[Decoder] Stream state created successfully"
        );

        Ok(StreamState {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
            duration,
            position_samples: 0,
            time_base,
        })
    }

    /// Convert Symphonia audio buffer to our `AudioBuffer` format
    ///
    /// Always outputs interleaved stereo f32 samples in the range [-1.0, 1.0].
    /// Multi-channel audio is downmixed to stereo using ITU-R BS.775-1 coefficients.
    fn convert_buffer(decoded: AudioBufferRef, sample_rate: u32) -> Result<AudioBuffer> {
        // Get channel count
        let channels = decoded.spec().channels.count();

        // Convert to f32 samples (interleaved stereo)
        // Uses symmetric scaling for signed integers (divide by 2^(N-1) not 2^(N-1)-1)
        // to ensure -1.0 to 1.0 range is symmetric
        let samples: Vec<f32> = match decoded {
            AudioBufferRef::F32(buf) => {
                // Already f32, clamp and interleave channels
                // F32 audio can have intersample peaks > 1.0, so we clamp
                Self::convert_multichannel_to_stereo(&buf, channels, |s| s.clamp(-1.0, 1.0))
            }
            AudioBufferRef::F64(buf) => {
                // Convert f64 to f32, clamp, and interleave
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    (s as f32).clamp(-1.0, 1.0)
                })
            }
            AudioBufferRef::S32(buf) => {
                // Convert i32 to f32 using symmetric scaling (divide by 2^31)
                // i32 range: -2147483648 to 2147483647
                // Dividing by 2147483648.0 gives symmetric [-1.0, 1.0) range
                Self::convert_multichannel_to_stereo(&buf, channels, |s| s as f32 / 2147483648.0)
            }
            AudioBufferRef::S16(buf) => {
                // Convert i16 to f32 using symmetric scaling (divide by 2^15)
                // i16 range: -32768 to 32767
                // Dividing by 32768.0 gives symmetric [-1.0, 1.0) range
                Self::convert_multichannel_to_stereo(&buf, channels, |s| s as f32 / 32768.0)
            }
            AudioBufferRef::S8(buf) => {
                // Convert i8 to f32 using symmetric scaling (divide by 2^7)
                // i8 range: -128 to 127
                // Dividing by 128.0 gives symmetric [-1.0, 1.0) range
                Self::convert_multichannel_to_stereo(&buf, channels, |s| s as f32 / 128.0)
            }
            AudioBufferRef::U32(buf) => {
                // Convert u32 to f32 and center around 0
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    (s as f32 / u32::MAX as f32) * 2.0 - 1.0
                })
            }
            AudioBufferRef::U16(buf) => {
                // Convert u16 to f32 and center around 0
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    (s as f32 / u16::MAX as f32) * 2.0 - 1.0
                })
            }
            AudioBufferRef::U8(buf) => {
                // Convert u8 to f32 and center around 0
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    (s as f32 / u8::MAX as f32) * 2.0 - 1.0
                })
            }
            AudioBufferRef::U24(buf) => {
                // Convert u24 to f32 and center around 0
                // U24 range: 0 to 16777215 (2^24 - 1)
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    (s.inner() as f32 / 16777215.0) * 2.0 - 1.0
                })
            }
            AudioBufferRef::S24(buf) => {
                // Convert i24 to f32 using symmetric scaling (divide by 2^23)
                // S24 range: -8388608 to 8388607
                // Dividing by 8388608.0 gives symmetric [-1.0, 1.0) range
                Self::convert_multichannel_to_stereo(&buf, channels, |s| {
                    s.inner() as f32 / 8388608.0
                })
            }
        };

        // Output is always stereo (2 channels) since we downmix
        let format = AudioFormat::new(SampleRate::new(sample_rate), 2, 32); // 32-bit float stereo

        Ok(AudioBuffer::new(samples, format))
    }

    /// Convert multi-channel audio to interleaved stereo with proper downmixing
    ///
    /// Uses ITU-R BS.775-1 downmix coefficients for surround sound:
    /// - L_out = L + 0.707*C + 0.707*Ls
    /// - R_out = R + 0.707*C + 0.707*Rs
    ///
    /// For 5.1 channel layout (FL, FR, C, LFE, SL, SR):
    /// - Channels 0,1: Front Left/Right -> direct to L/R
    /// - Channel 2: Center -> 0.707 to both L and R
    /// - Channel 3: LFE -> 0.707 to both L and R (or can be omitted)
    /// - Channels 4,5: Surround Left/Right -> 0.707 to L/R respectively
    fn convert_multichannel_to_stereo<T, F>(
        buf: &symphonia::core::audio::AudioBuffer<T>,
        channels: usize,
        normalize: F,
    ) -> Vec<f32>
    where
        T: symphonia::core::sample::Sample + Copy,
        F: Fn(T) -> f32,
    {
        let frames = buf.frames();
        let mut output = Vec::with_capacity(frames * 2);

        // ITU-R BS.775-1 coefficient for center and surround channels
        const CENTER_MIX: f32 = 0.707; // -3dB

        match channels {
            0 => {
                // No channels - fill with silence
                output.resize(frames * 2, 0.0);
            }
            1 => {
                // Mono - duplicate to both channels
                let mono = buf.chan(0);
                for &sample_val in mono.iter().take(frames) {
                    let sample = normalize(sample_val);
                    output.push(sample);
                    output.push(sample);
                }
            }
            2 => {
                // Stereo - direct pass-through
                let left = buf.chan(0);
                let right = buf.chan(1);
                for i in 0..frames {
                    output.push(normalize(left[i]));
                    output.push(normalize(right[i]));
                }
            }
            3 => {
                // 3 channels (L, R, C) - mix center into both
                let left = buf.chan(0);
                let right = buf.chan(1);
                let center = buf.chan(2);
                for i in 0..frames {
                    let l = normalize(left[i]);
                    let r = normalize(right[i]);
                    let c = normalize(center[i]) * CENTER_MIX;
                    output.push((l + c).clamp(-1.0, 1.0));
                    output.push((r + c).clamp(-1.0, 1.0));
                }
            }
            4 => {
                // 4 channels (L, R, SL, SR) - quad layout
                let left = buf.chan(0);
                let right = buf.chan(1);
                let surround_left = buf.chan(2);
                let surround_right = buf.chan(3);
                for i in 0..frames {
                    let l = normalize(left[i]);
                    let r = normalize(right[i]);
                    let sl = normalize(surround_left[i]) * CENTER_MIX;
                    let sr = normalize(surround_right[i]) * CENTER_MIX;
                    output.push((l + sl).clamp(-1.0, 1.0));
                    output.push((r + sr).clamp(-1.0, 1.0));
                }
            }
            5 => {
                // 5 channels (L, R, C, SL, SR) - 5.0 layout
                let left = buf.chan(0);
                let right = buf.chan(1);
                let center = buf.chan(2);
                let surround_left = buf.chan(3);
                let surround_right = buf.chan(4);
                for i in 0..frames {
                    let l = normalize(left[i]);
                    let r = normalize(right[i]);
                    let c = normalize(center[i]) * CENTER_MIX;
                    let sl = normalize(surround_left[i]) * CENTER_MIX;
                    let sr = normalize(surround_right[i]) * CENTER_MIX;
                    output.push((l + c + sl).clamp(-1.0, 1.0));
                    output.push((r + c + sr).clamp(-1.0, 1.0));
                }
            }
            _ => {
                // 6+ channels (L, R, C, LFE, SL, SR, ...) - 5.1 or higher
                // Standard 5.1 layout: FL, FR, C, LFE, SL, SR
                let left = buf.chan(0);
                let right = buf.chan(1);
                let center = buf.chan(2);
                let lfe = buf.chan(3);
                let surround_left = buf.chan(4);
                let surround_right = if channels > 5 {
                    buf.chan(5)
                } else {
                    buf.chan(4) // Fallback if only 6 channels but index issue
                };
                for i in 0..frames {
                    let l = normalize(left[i]);
                    let r = normalize(right[i]);
                    let c = normalize(center[i]) * CENTER_MIX;
                    let lfe_sample = normalize(lfe[i]) * CENTER_MIX;
                    let sl = normalize(surround_left[i]) * CENTER_MIX;
                    let sr = normalize(surround_right[i]) * CENTER_MIX;
                    // Mix all channels to stereo
                    output.push((l + c + lfe_sample + sl).clamp(-1.0, 1.0));
                    output.push((r + c + lfe_sample + sr).clamp(-1.0, 1.0));
                }
            }
        }

        output
    }

    /// Interleave two channels into a single buffer
    fn interleave_f32(left: &[f32], right: &[f32]) -> Vec<f32> {
        let mut interleaved = Vec::with_capacity(left.len() + right.len());
        for (l, r) in left.iter().zip(right.iter()) {
            interleaved.push(*l);
            interleaved.push(*r);
        }
        interleaved
    }
}

impl Default for SymphoniaDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SymphoniaDecoder {
    /// Close any open streaming session
    pub fn close(&mut self) {
        self.stream_state = None;
    }

    /// Check if a file is currently open for streaming
    pub fn is_open(&self) -> bool {
        self.stream_state.is_some()
    }

    /// Extract metadata from an audio file
    ///
    /// This extracts all available metadata from the file including:
    /// - Tag information (title, artist, album, etc.)
    /// - Audio properties (sample rate, channels, duration)
    /// - Embedded album art
    /// - Extended metadata (MusicBrainz IDs, ReplayGain, etc.)
    ///
    /// This is more efficient than `decode()` as it only reads the metadata
    /// without decoding the entire audio stream.
    ///
    /// # Arguments
    /// * `path` - Path to the audio file
    ///
    /// # Returns
    /// * `FileMetadata` with all available metadata
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or probed
    pub fn extract_metadata(&self, path: &Path) -> Result<FileMetadata> {
        metadata::extract_metadata(path)
    }
}

impl AudioDecoderTrait for SymphoniaDecoder {
    fn decode(&mut self, path: &Path) -> soul_core::Result<AudioBuffer> {
        tracing::info!(file_path = %path.display(), "[Decoder] Starting full file decode");
        let decode_start = std::time::Instant::now();

        // Check if file exists
        if !path.exists() {
            tracing::error!(file_path = %path.display(), "[Decoder] File not found");
            return Err(AudioError::FileNotFound(path.display().to_string()).into());
        }

        // Open the file
        let file =
            std::fs::File::open(path).map_err(|e| {
                tracing::error!(file_path = %path.display(), error = %e, "[Decoder] Failed to open file");
                soul_core::SoulError::audio(e.to_string())
            })?;

        // Create media source
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| soul_core::SoulError::audio(format!("Failed to probe file: {}", e)))?;

        let mut format = probed.format;

        // Find the default track
        let track = format
            .default_track()
            .ok_or_else(|| soul_core::SoulError::audio("No audio tracks found"))?;

        // Get sample rate and track ID before entering loop
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let track_id = track.id;

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| {
                tracing::error!(error = %e, "[Decoder] Failed to create decoder");
                soul_core::SoulError::audio(format!("Failed to create decoder: {}", e))
            })?;

        tracing::debug!(
            sample_rate,
            track_id,
            codec = ?track.codec_params.codec,
            "[Decoder] Starting packet decoding loop"
        );

        // Decode all packets and collect into single buffer
        let mut all_samples = Vec::new();
        let mut packet_count = 0;
        let decode_loop_start = std::time::Instant::now();

        loop {
            // Get the next packet
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        packets_decoded = packet_count,
                        "[Decoder] Error reading packet"
                    );
                    return Err(soul_core::SoulError::audio(format!(
                        "Error reading packet: {}",
                        e
                    )));
                }
            };

            // Skip packets that are not for the default track
            if packet.track_id() != track_id {
                continue;
            }

            packet_count += 1;

            // Decode the packet
            let decoded = decoder.decode(&packet).map_err(|e| {
                tracing::error!(
                    error = %e,
                    packet_num = packet_count,
                    "[Decoder] Packet decode error"
                );
                soul_core::SoulError::audio(format!("Decode error: {}", e))
            })?;

            // Convert and append to buffer (always outputs stereo)
            let buffer = Self::convert_buffer(decoded, sample_rate)?;
            all_samples.extend_from_slice(&buffer.samples);

            // Log slow packet processing
            if packet_count % 1000 == 0 {
                let elapsed = decode_loop_start.elapsed();
                if elapsed.as_secs() > 1 {
                    tracing::warn!(
                        packets_decoded = packet_count,
                        elapsed_secs = elapsed.as_secs(),
                        "[Decoder] Slow decode detected"
                    );
                }
            }
        }

        let decode_loop_duration = decode_loop_start.elapsed();

        // Output is always stereo (2 channels) since convert_buffer downmixes
        let format = AudioFormat::new(SampleRate::new(sample_rate), 2, 32);

        let total_duration = decode_start.elapsed();
        let total_frames = all_samples.len() / 2;
        let duration_secs = total_frames as f64 / sample_rate as f64;

        tracing::info!(
            file_path = %path.display(),
            packet_count,
            total_frames,
            duration_secs = format!("{:.2}", duration_secs),
            sample_rate,
            decode_loop_ms = decode_loop_duration.as_millis(),
            total_duration_ms = total_duration.as_millis(),
            "[Decoder] Full file decode completed"
        );

        Ok(AudioBuffer::new(all_samples, format))
    }

    fn supports_format(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "mp3" | "flac" | "ogg" | "opus" | "wav" | "m4a" | "aac"
            )
        } else {
            false
        }
    }

    fn open(&mut self, path: &Path) -> soul_core::Result<AudioMetadata> {
        // Close any existing stream
        self.stream_state = None;

        // Create new stream state
        let state = Self::create_stream_state(path)?;

        let metadata = AudioMetadata {
            sample_rate: state.sample_rate,
            channels: state.channels,
            duration: state.duration,
            bits_per_sample: None, // Could extract from codec_params if needed
        };

        self.stream_state = Some(state);
        Ok(metadata)
    }

    fn decode_chunk(&mut self, max_frames: usize) -> soul_core::Result<Option<AudioBuffer>> {
        let state = self.stream_state.as_mut().ok_or(AudioError::NoFileOpen)?;

        // Try to decode packets until we have enough frames or reach EOF
        let mut all_samples = Vec::new();
        let target_samples = max_frames * 2; // Stereo output

        loop {
            // Get the next packet
            let packet = match state.format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    // End of file reached
                    break;
                }
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    // Decoder needs reset (can happen after seek)
                    state.decoder.reset();
                    continue;
                }
                Err(e) => {
                    return Err(
                        AudioError::Symphonia(format!("Error reading packet: {}", e)).into(),
                    );
                }
            };

            // Skip packets that are not for our track
            if packet.track_id() != state.track_id {
                continue;
            }

            // Decode the packet
            let decoded = match state.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(symphonia::core::errors::Error::DecodeError(e)) => {
                    // Some decode errors can be recovered from
                    tracing::warn!(
                        error = %e,
                        "[SymphoniaDecoder] Decode error (recoverable)"
                    );
                    continue;
                }
                Err(e) => {
                    return Err(AudioError::DecodeError(format!("Decode error: {}", e)).into());
                }
            };

            // Update position based on decoded frames
            let frames_decoded = decoded.frames() as u64;
            state.position_samples += frames_decoded;

            // Convert to stereo f32
            let buffer = Self::convert_buffer(decoded, state.sample_rate)?;
            all_samples.extend_from_slice(&buffer.samples);

            // Check if we have enough samples
            if all_samples.len() >= target_samples {
                break;
            }
        }

        if all_samples.is_empty() {
            return Ok(None);
        }

        // Truncate to max_frames if we decoded more
        if all_samples.len() > target_samples {
            all_samples.truncate(target_samples);
        }

        let format = AudioFormat::new(SampleRate::new(state.sample_rate), 2, 32);
        Ok(Some(AudioBuffer::new(all_samples, format)))
    }

    fn seek(&mut self, position: Duration) -> soul_core::Result<Duration> {
        let state = self.stream_state.as_mut().ok_or(AudioError::NoFileOpen)?;

        // Clamp position to valid range
        // - Minimum: 0 (start of track)
        // - Maximum: duration - 1ms (to avoid seeking exactly to EOF)
        let clamped_position = if let Some(duration) = state.duration {
            // Leave 1ms margin before end to avoid EOF edge case
            let max_position = duration.saturating_sub(Duration::from_millis(1));
            position.min(max_position)
        } else {
            position
        };

        // Log edge case seeks for debugging
        if position.as_millis() < 100 {
            tracing::debug!(
                position_ms = position.as_millis(),
                "[Decoder] Seek near start (0-100ms)"
            );
        } else if let Some(duration) = state.duration {
            let remaining = duration.saturating_sub(position);
            if remaining.as_millis() < 100 {
                tracing::debug!(
                    position_ms = position.as_millis(),
                    duration_ms = duration.as_millis(),
                    remaining_ms = remaining.as_millis(),
                    "[Decoder] Seek near end (last 100ms)"
                );
            }
        }

        // Convert duration to Time for Symphonia
        let secs = clamped_position.as_secs();
        let frac = (clamped_position.as_nanos() % 1_000_000_000) as f64 / 1_000_000_000.0;

        let time = Time::new(secs, frac);

        // Perform seek using accurate mode
        let seek_result = state.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time,
                track_id: Some(state.track_id),
            },
        );

        match seek_result {
            Ok(seeked_to) => {
                // Reset decoder state after seek
                state.decoder.reset();

                // Calculate actual position from timestamp
                let actual_position = if let Some(tb) = state.time_base {
                    let ts = seeked_to.actual_ts;
                    Duration::from_secs_f64(ts as f64 * tb.numer as f64 / tb.denom as f64)
                } else {
                    // Fallback: assume sample-based timestamp
                    Duration::from_secs_f64(seeked_to.actual_ts as f64 / state.sample_rate as f64)
                };

                // Update position tracking using high-precision calculation
                // Use u128 intermediate to avoid overflow for very long files
                let sample_rate_u64 = state.sample_rate as u64;
                let position_nanos = actual_position.as_nanos();
                state.position_samples =
                    ((position_nanos * sample_rate_u64 as u128) / 1_000_000_000) as u64;

                // Calculate seek accuracy (difference between requested and actual)
                // Use saturating_sub to handle potential underflow gracefully
                let seek_error_ms = actual_position
                    .saturating_sub(clamped_position)
                    .max(clamped_position.saturating_sub(actual_position))
                    .as_millis();

                tracing::debug!(
                    requested_position_ms = clamped_position.as_millis(),
                    actual_position_ms = actual_position.as_millis(),
                    seek_error_ms = seek_error_ms,
                    "[Decoder] Seek completed"
                );

                // Warn if seek accuracy is poor (> 50ms difference)
                // This can happen with frame-based formats like MP3/AAC
                if seek_error_ms > 50 {
                    tracing::warn!(
                        requested_ms = clamped_position.as_millis(),
                        actual_ms = actual_position.as_millis(),
                        error_ms = seek_error_ms,
                        "[Decoder] Seek accuracy issue: landed {}ms from target",
                        seek_error_ms
                    );
                }

                Ok(actual_position)
            }
            Err(symphonia::core::errors::Error::SeekError(kind)) => {
                tracing::warn!(
                    requested_position_ms = clamped_position.as_millis(),
                    error = ?kind,
                    "[Decoder] Seek failed"
                );
                Err(AudioError::SeekError(format!("Seek failed: {:?}", kind)).into())
            }
            Err(e) => {
                tracing::warn!(
                    requested_position_ms = clamped_position.as_millis(),
                    error = %e,
                    "[Decoder] Seek error"
                );
                Err(AudioError::SeekError(format!("Seek error: {}", e)).into())
            }
        }
    }

    fn duration(&self) -> Option<Duration> {
        self.stream_state.as_ref().and_then(|s| s.duration)
    }

    fn position(&self) -> Duration {
        self.stream_state
            .as_ref()
            .map(|s| Duration::from_secs_f64(s.position_samples as f64 / s.sample_rate as f64))
            .unwrap_or(Duration::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_creation() {
        let decoder = SymphoniaDecoder::new();
        drop(decoder);
    }

    #[test]
    fn supports_common_formats() {
        let decoder = SymphoniaDecoder::new();
        assert!(decoder.supports_format(Path::new("test.mp3")));
        assert!(decoder.supports_format(Path::new("test.flac")));
        assert!(decoder.supports_format(Path::new("test.ogg")));
        assert!(decoder.supports_format(Path::new("test.wav")));
        assert!(!decoder.supports_format(Path::new("test.txt")));
    }

    #[test]
    fn decode_nonexistent_file_returns_error() {
        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }
}
