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
//! The design separates format-specific concerns from playback logic:
//!
//! 1. **Generic Interleaving** (`interleave_to_stereo_f32`):
//!    - Takes any sample type `T` and a normalization function
//!    - Converts planar audio to interleaved stereo f32
//!    - Handles mono→stereo duplication automatically
//!
//! 2. **Format Conversion** (`convert_to_f32_interleaved`):
//!    - Matches on `AudioBufferRef` variant
//!    - Provides format-specific normalization:
//!      - **Float formats**: Pass through (F32) or cast (F64)
//!      - **Signed ints**: Divide by MAX value
//!      - **Unsigned ints**: Normalize to [0,1], scale to [-1,1]
//!      - **24-bit types**: Extract `.inner()`, normalize
//!
//! This approach ensures:
//! - All formats use identical interleaving logic (no duplication)
//! - Only normalization parameters change per format
//! - Easy to add new formats (just add match arm with normalization function)
//! - Compile-time type safety for all conversions

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use soul_playback::{AudioSource, PlaybackError, Result};
use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;

/// Size of ring buffer in seconds
const BUFFER_SIZE_SECONDS: usize = 5;

/// Audio source for local files with streaming decoder
///
/// Uses Symphonia to decode audio files from disk on-demand.
/// Maintains a small ring buffer (5 seconds) for smooth playback.
/// Fast startup - only decodes metadata initially, then streams packets.
/// Automatically resamples audio to match target sample rate.
///
/// Supports all formats: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct LocalAudioSource {
    path: PathBuf,
    source_sample_rate: u32, // Sample rate of the audio file
    target_sample_rate: u32, // Target output sample rate
    channels: u16,

    // Symphonia streaming components
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    time_base: TimeBase,

    // Ring buffer for decoded samples (at SOURCE rate, before resampling)
    input_buffer: VecDeque<f32>,

    // Ring buffer for resampled samples (at TARGET rate, ready for output)
    output_buffer: VecDeque<f32>,
    output_buffer_capacity: usize, // Max samples to buffer

    // Resampler (if needed)
    resampler: Option<SincFixedIn<f32>>,
    needs_resampling: bool,
    resampler_chunk_frames: usize, // Number of frames needed per resample chunk

    // Position tracking
    samples_decoded: usize, // Total samples decoded from start (at source rate)
    samples_read: usize,    // Total samples read by audio callback (at target rate)
    total_duration: Duration,
    is_eof: bool,
}

impl LocalAudioSource {
    /// Create a new streaming local audio source
    ///
    /// Only decodes metadata and first packet for fast startup.
    /// Subsequent packets are decoded on-demand during playback.
    /// Automatically resamples audio if the file's sample rate doesn't match the target.
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

        // Open the file
        let file = File::open(&path)
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to open file: {}", e)))?;

        // Create media source stream
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create hint for format detection
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // Probe the file to detect format
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to probe file: {}", e)))?;

        let format_reader = probed.format;

        // Find the default audio track
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

        eprintln!("[LocalAudioSource] File info:");
        eprintln!("  - Path: {}", path.display());
        eprintln!("  - Source sample rate: {} Hz", sample_rate);
        eprintln!("  - Target sample rate: {} Hz", target_sample_rate);
        eprintln!("  - Channels: {}", channels);
        eprintln!(
            "  - Needs resampling: {}",
            sample_rate != target_sample_rate
        );
        if sample_rate != target_sample_rate {
            let resample_ratio = target_sample_rate as f64 / sample_rate as f64;
            eprintln!(
                "  - Resample ratio: {:.6} ({}x)",
                resample_ratio,
                if resample_ratio > 1.0 {
                    "upsampling"
                } else {
                    "downsampling"
                }
            );
        }

        // Calculate total duration if available
        // If n_frames is unknown, use Duration::MAX to indicate unknown duration.
        // This is more honest than an arbitrary 180s default and allows UI to
        // display "unknown" instead of a misleading progress bar.
        let total_duration = track
            .codec_params
            .n_frames
            .map(|frames| Duration::from_secs_f64(frames as f64 / sample_rate as f64))
            .unwrap_or(Duration::MAX);

        // Create decoder for this track
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to create decoder: {}", e)))?;

        // Calculate buffer capacity (5 seconds of stereo audio at target sample rate)
        let output_buffer_capacity =
            (BUFFER_SIZE_SECONDS * target_sample_rate as usize) * channels as usize;

        // Check if resampling is needed
        let needs_resampling = sample_rate != target_sample_rate;

        // Use a smaller chunk size for lower latency (1024 frames is common)
        // This must match what we pass to the resampler
        let resampler_chunk_frames = 1024;

        let resampler = if needs_resampling {
            // Create resampler for streaming (using SincFixedIn with fixed input chunk size)
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let resample_ratio = target_sample_rate as f64 / sample_rate as f64;
            eprintln!("[LocalAudioSource] Creating resampler:");
            eprintln!("  - Ratio: {:.6}", resample_ratio);
            eprintln!("  - Chunk frames: {}", resampler_chunk_frames);
            eprintln!("  - Channels: {}", channels);

            match SincFixedIn::<f32>::new(
                resample_ratio,
                2.0, // max relative ratio change
                params,
                resampler_chunk_frames,
                channels as usize,
            ) {
                Ok(resampler) => {
                    eprintln!("[LocalAudioSource] Resampler created successfully");
                    eprintln!("  - Input frames needed: {}", resampler.input_frames_next());
                    Some(resampler)
                }
                Err(e) => {
                    return Err(PlaybackError::AudioSource(format!(
                        "Failed to create resampler: {}",
                        e
                    )));
                }
            }
        } else {
            None
        };

        // Pre-allocate input buffer for accumulating samples before resampling
        // Size: enough for several chunks worth of samples
        let input_buffer_capacity = resampler_chunk_frames * channels as usize * 4;

        Ok(Self {
            path,
            source_sample_rate: sample_rate,
            target_sample_rate,
            channels,
            format_reader,
            decoder,
            track_id,
            time_base,
            input_buffer: VecDeque::with_capacity(input_buffer_capacity),
            output_buffer: VecDeque::with_capacity(output_buffer_capacity),
            output_buffer_capacity,
            resampler,
            needs_resampling,
            resampler_chunk_frames,
            samples_decoded: 0,
            samples_read: 0,
            total_duration,
            is_eof: false,
        })
    }

    /// Decode next packet and add samples to the appropriate buffer
    ///
    /// If resampling is needed:
    /// - Decoded samples go to input_buffer
    /// - When input_buffer has enough frames, resample and move to output_buffer
    ///
    /// If no resampling:
    /// - Decoded samples go directly to output_buffer
    fn decode_next_packet(&mut self) -> Result<bool> {
        if self.is_eof {
            return Ok(false);
        }

        // Get next packet from format reader
        let packet = match self.format_reader.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                self.is_eof = true;
                // Process any remaining samples in input buffer
                if self.needs_resampling {
                    self.flush_resampler()?;
                }
                return Ok(false);
            }
            Err(e) => {
                return Err(PlaybackError::AudioSource(format!(
                    "Error reading packet: {}",
                    e
                )));
            }
        };

        // Skip packets from other tracks
        if packet.track_id() != self.track_id {
            return Ok(true); // Try next packet
        }

        // Decode the packet
        let decoded = self
            .decoder
            .decode(&packet)
            .map_err(|e| PlaybackError::AudioSource(format!("Decode error: {}", e)))?;

        // Convert to f32 samples
        let samples = Self::convert_to_f32_interleaved(decoded, self.channels)?;
        let samples_len = samples.len();

        if self.needs_resampling {
            // Add samples to input buffer (at source rate)
            for sample in samples {
                self.input_buffer.push_back(sample);
            }

            // Process resampling when we have enough samples
            self.process_resampling()?;
        } else {
            // No resampling needed - add directly to output buffer
            for sample in samples {
                self.output_buffer.push_back(sample);

                // Enforce buffer size limit
                if self.output_buffer.len() > self.output_buffer_capacity {
                    self.output_buffer.pop_front();
                }
            }
        }

        self.samples_decoded += samples_len;

        Ok(true)
    }

    /// Process resampling: convert samples from input_buffer to output_buffer
    ///
    /// Rubato's SincFixedIn requires a fixed number of input frames per call.
    /// We accumulate samples in input_buffer until we have enough for a chunk,
    /// then resample the chunk and add to output_buffer.
    fn process_resampling(&mut self) -> Result<()> {
        let Some(ref mut resampler) = self.resampler else {
            return Ok(());
        };

        let channels = self.channels as usize;
        let chunk_frames = self.resampler_chunk_frames;
        let samples_per_chunk = chunk_frames * channels;

        // Process all complete chunks we have
        while self.input_buffer.len() >= samples_per_chunk {
            // Deinterleave chunk for rubato (rubato expects Vec<Vec<f32>> for each channel)
            let mut deinterleaved: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk_frames); channels];

            for _ in 0..chunk_frames {
                for ch in 0..channels {
                    let sample = self.input_buffer.pop_front().unwrap();
                    deinterleaved[ch].push(sample);
                }
            }

            // Resample chunk
            let resampled = resampler
                .process(&deinterleaved, None)
                .map_err(|e| PlaybackError::AudioSource(format!("Resampling error: {}", e)))?;

            // Interleave resampled chunk and add to output buffer
            let output_frames = resampled[0].len();
            for frame_idx in 0..output_frames {
                for ch in 0..channels {
                    self.output_buffer.push_back(resampled[ch][frame_idx]);

                    // Enforce buffer size limit
                    if self.output_buffer.len() > self.output_buffer_capacity {
                        self.output_buffer.pop_front();
                    }
                }
            }
        }

        Ok(())
    }

    /// Flush remaining samples through the resampler at end of file
    ///
    /// When we reach EOF, there may be samples left in input_buffer that
    /// don't form a complete chunk. We need to pad them and process.
    fn flush_resampler(&mut self) -> Result<()> {
        let Some(ref mut resampler) = self.resampler else {
            return Ok(());
        };

        if self.input_buffer.is_empty() {
            return Ok(());
        }

        let channels = self.channels as usize;
        let chunk_frames = self.resampler_chunk_frames;
        let remaining_samples = self.input_buffer.len();
        let remaining_frames = remaining_samples / channels;

        if remaining_frames == 0 {
            return Ok(());
        }

        // Deinterleave remaining samples
        let mut deinterleaved: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk_frames); channels];

        for _ in 0..remaining_frames {
            for ch in 0..channels {
                let sample = self.input_buffer.pop_front().unwrap();
                deinterleaved[ch].push(sample);
            }
        }

        // Pad with zeros to make a complete chunk
        let frames_to_pad = chunk_frames - remaining_frames;
        for ch in 0..channels {
            deinterleaved[ch].extend(std::iter::repeat(0.0f32).take(frames_to_pad));
        }

        // Process the padded chunk
        let resampled = resampler
            .process(&deinterleaved, None)
            .map_err(|e| PlaybackError::AudioSource(format!("Resampling error: {}", e)))?;

        // Only take the valid output frames (proportional to input)
        let output_frames = resampled[0].len();
        let valid_output_frames =
            (remaining_frames as f64 / chunk_frames as f64 * output_frames as f64) as usize;

        for frame_idx in 0..valid_output_frames {
            for ch in 0..channels {
                self.output_buffer.push_back(resampled[ch][frame_idx]);
            }
        }

        Ok(())
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
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize> {
        let mut samples_written = 0;

        while samples_written < output.len() {
            // If output buffer is running low, decode more packets
            if self.output_buffer.len() < output.len() && !self.is_eof {
                // Decode packets until output buffer is full or EOF
                while self.output_buffer.len() < self.output_buffer_capacity && !self.is_eof {
                    if !self.decode_next_packet()? {
                        break;
                    }
                }
            }

            // Copy from output buffer to output
            let available = self.output_buffer.len().min(output.len() - samples_written);
            if available == 0 {
                // No more data available
                break;
            }

            for i in 0..available {
                output[samples_written + i] = self.output_buffer.pop_front().unwrap();
            }

            samples_written += available;
            self.samples_read += available;
        }

        // Fill remainder with silence if needed
        if samples_written < output.len() {
            output[samples_written..].fill(0.0);
        }

        Ok(samples_written)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.total_duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }

        // Convert duration to Symphonia time units
        let seek_ts = self.time_base.calc_timestamp(position.into());

        // Perform seek
        self.format_reader
            .seek(
                symphonia::core::formats::SeekMode::Accurate,
                symphonia::core::formats::SeekTo::TimeStamp {
                    ts: seek_ts,
                    track_id: self.track_id,
                },
            )
            .map_err(|e| PlaybackError::AudioSource(format!("Seek failed: {}", e)))?;

        // Reset decoder state
        self.decoder.reset();

        // Clear both buffers and reset position tracking
        self.input_buffer.clear();
        self.output_buffer.clear();
        // samples_read is at TARGET sample rate (output)
        self.samples_read = (position.as_secs_f64()
            * self.target_sample_rate as f64
            * self.channels as f64) as usize;
        // samples_decoded is at SOURCE sample rate (input from decoder)
        // This is different from samples_read when resampling is active
        self.samples_decoded = (position.as_secs_f64()
            * self.source_sample_rate as f64
            * self.channels as f64) as usize;
        self.is_eof = false;

        // Reset resampler if needed
        if let Some(ref mut resampler) = self.resampler {
            resampler.reset();
        }

        Ok(())
    }

    fn duration(&self) -> Duration {
        self.total_duration
    }

    fn position(&self) -> Duration {
        // Calculate position based on samples read (at target sample rate)
        let frames = self.samples_read / self.channels as usize;
        Duration::from_secs_f64(frames as f64 / self.target_sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        self.is_eof && self.output_buffer.is_empty() && self.input_buffer.is_empty()
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

        // Read 1 second worth of samples from each
        let samples_44100_per_sec = 44100 * 2; // stereo
        let samples_48000_per_sec = 48000 * 2; // stereo

        let mut buffer_44100 = vec![0.0f32; samples_44100_per_sec];
        let mut buffer_48000 = vec![0.0f32; samples_48000_per_sec];

        let read_44100 = source_44100
            .read_samples(&mut buffer_44100)
            .expect("Failed to read at 44.1kHz");
        let read_48000 = source_48000
            .read_samples(&mut buffer_48000)
            .expect("Failed to read at 48kHz");

        // Should have read the full buffers (1 second worth)
        assert_eq!(
            read_44100, samples_44100_per_sec,
            "Should read 1 second at 44.1kHz"
        );
        assert_eq!(
            read_48000, samples_48000_per_sec,
            "Should read 1 second at 48kHz"
        );

        // Position should be approximately 1 second for both
        let pos_44100 = source_44100.position().as_secs_f64();
        let pos_48000 = source_48000.position().as_secs_f64();

        assert!(
            (pos_44100 - 1.0).abs() < 0.1,
            "Position after reading 1 second should be ~1.0s at 44.1kHz (got {:.2}s)",
            pos_44100
        );
        assert!(
            (pos_48000 - 1.0).abs() < 0.1,
            "Position after reading 1 second should be ~1.0s at 48kHz (got {:.2}s)",
            pos_48000
        );
    }
}
