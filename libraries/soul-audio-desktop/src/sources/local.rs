//! Local file audio source using Symphonia decoder with streaming

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
///
/// Supports all formats: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct LocalAudioSource {
    path: PathBuf,
    sample_rate: u32,
    channels: u16,

    // Symphonia streaming components
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    time_base: TimeBase,

    // Ring buffer for decoded samples
    buffer: VecDeque<f32>,
    buffer_capacity: usize, // Max samples to buffer

    // Position tracking
    samples_decoded: usize, // Total samples decoded from start
    samples_read: usize,    // Total samples read by audio callback
    total_duration: Duration,
    is_eof: bool,
}

impl LocalAudioSource {
    /// Create a new streaming local audio source
    ///
    /// Only decodes metadata and first packet for fast startup.
    /// Subsequent packets are decoded on-demand during playback.
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    ///
    /// # Returns
    /// * `Ok(source)` - Audio source ready for streaming playback
    /// * `Err(_)` - Failed to open or probe file
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
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

        // Calculate total duration if available
        let total_duration = track
            .codec_params
            .n_frames
            .map(|frames| Duration::from_secs_f64(frames as f64 / sample_rate as f64))
            .unwrap_or(Duration::from_secs(180)); // Default to 3 minutes if unknown

        // Create decoder for this track
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to create decoder: {}", e)))?;

        // Calculate buffer capacity (5 seconds of stereo audio)
        let buffer_capacity = (BUFFER_SIZE_SECONDS * sample_rate as usize) * channels as usize;

        Ok(Self {
            path,
            sample_rate,
            channels,
            format_reader,
            decoder,
            track_id,
            time_base,
            buffer: VecDeque::with_capacity(buffer_capacity),
            buffer_capacity,
            samples_decoded: 0,
            samples_read: 0,
            total_duration,
            is_eof: false,
        })
    }

    /// Decode next packet and add samples to ring buffer
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

        // Convert to f32 samples (borrowing decoded, not self)
        let samples = Self::convert_to_f32_interleaved(decoded, self.channels)?;

        // Now append samples to buffer
        for sample in samples {
            self.buffer.push_back(sample);

            // Enforce buffer size limit
            if self.buffer.len() > self.buffer_capacity {
                self.buffer.pop_front();
            }
        }

        self.samples_decoded += self.buffer.len();

        Ok(true)
    }

    /// Convert Symphonia AudioBufferRef to interleaved f32 samples
    fn convert_to_f32_interleaved(
        decoded: AudioBufferRef,
        target_channels: u16,
    ) -> Result<Vec<f32>> {
        let channels = decoded.spec().channels.count();
        let frames = decoded.frames();

        let mut output = Vec::with_capacity(frames * target_channels as usize);

        match decoded {
            AudioBufferRef::F32(buf) => {
                for frame_idx in 0..frames {
                    // Left channel
                    output.push(buf.chan(0)[frame_idx]);
                    // Right channel (duplicate left if mono)
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx]);
                    } else {
                        output.push(buf.chan(0)[frame_idx]);
                    }
                }
            }
            AudioBufferRef::S16(buf) => {
                for frame_idx in 0..frames {
                    output.push(buf.chan(0)[frame_idx] as f32 / i16::MAX as f32);
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx] as f32 / i16::MAX as f32);
                    } else {
                        output.push(buf.chan(0)[frame_idx] as f32 / i16::MAX as f32);
                    }
                }
            }
            AudioBufferRef::S32(buf) => {
                for frame_idx in 0..frames {
                    output.push(buf.chan(0)[frame_idx] as f32 / i32::MAX as f32);
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx] as f32 / i32::MAX as f32);
                    } else {
                        output.push(buf.chan(0)[frame_idx] as f32 / i32::MAX as f32);
                    }
                }
            }
            AudioBufferRef::F64(buf) => {
                for frame_idx in 0..frames {
                    output.push(buf.chan(0)[frame_idx] as f32);
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx] as f32);
                    } else {
                        output.push(buf.chan(0)[frame_idx] as f32);
                    }
                }
            }
            AudioBufferRef::U8(buf) => {
                for frame_idx in 0..frames {
                    output.push((buf.chan(0)[frame_idx] as f32 / u8::MAX as f32) * 2.0 - 1.0);
                    if channels > 1 {
                        output.push((buf.chan(1)[frame_idx] as f32 / u8::MAX as f32) * 2.0 - 1.0);
                    } else {
                        output.push((buf.chan(0)[frame_idx] as f32 / u8::MAX as f32) * 2.0 - 1.0);
                    }
                }
            }
            AudioBufferRef::U16(buf) => {
                for frame_idx in 0..frames {
                    output.push((buf.chan(0)[frame_idx] as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    if channels > 1 {
                        output.push((buf.chan(1)[frame_idx] as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    } else {
                        output.push((buf.chan(0)[frame_idx] as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    }
                }
            }
            AudioBufferRef::U32(buf) => {
                for frame_idx in 0..frames {
                    output.push((buf.chan(0)[frame_idx] as f32 / u32::MAX as f32) * 2.0 - 1.0);
                    if channels > 1 {
                        output.push((buf.chan(1)[frame_idx] as f32 / u32::MAX as f32) * 2.0 - 1.0);
                    } else {
                        output.push((buf.chan(0)[frame_idx] as f32 / u32::MAX as f32) * 2.0 - 1.0);
                    }
                }
            }
            AudioBufferRef::S8(buf) => {
                for frame_idx in 0..frames {
                    output.push(buf.chan(0)[frame_idx] as f32 / i8::MAX as f32);
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx] as f32 / i8::MAX as f32);
                    } else {
                        output.push(buf.chan(0)[frame_idx] as f32 / i8::MAX as f32);
                    }
                }
            }
            AudioBufferRef::S24(buf) => {
                for frame_idx in 0..frames {
                    output.push(buf.chan(0)[frame_idx].inner() as f32 / 8388607.0);
                    if channels > 1 {
                        output.push(buf.chan(1)[frame_idx].inner() as f32 / 8388607.0);
                    } else {
                        output.push(buf.chan(0)[frame_idx].inner() as f32 / 8388607.0);
                    }
                }
            }
            AudioBufferRef::U24(buf) => {
                for frame_idx in 0..frames {
                    output.push((buf.chan(0)[frame_idx].inner() as f32 / 8388607.0) * 2.0 - 1.0);
                    if channels > 1 {
                        output
                            .push((buf.chan(1)[frame_idx].inner() as f32 / 8388607.0) * 2.0 - 1.0);
                    } else {
                        output
                            .push((buf.chan(0)[frame_idx].inner() as f32 / 8388607.0) * 2.0 - 1.0);
                    }
                }
            }
        }

        Ok(output)
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
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
            // If buffer is running low, decode more packets
            if self.buffer.len() < output.len() && !self.is_eof {
                // Decode packets until buffer is full or EOF
                while self.buffer.len() < self.buffer_capacity && !self.is_eof {
                    if !self.decode_next_packet()? {
                        break;
                    }
                }
            }

            // Copy from buffer to output
            let available = self.buffer.len().min(output.len() - samples_written);
            if available == 0 {
                // No more data available
                break;
            }

            for i in 0..available {
                output[samples_written + i] = self.buffer.pop_front().unwrap();
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
                    ts: seek_ts as u64,
                    track_id: self.track_id,
                },
            )
            .map_err(|e| PlaybackError::AudioSource(format!("Seek failed: {}", e)))?;

        // Reset decoder state
        self.decoder.reset();

        // Clear buffer and reset position tracking
        self.buffer.clear();
        self.samples_read =
            (position.as_secs_f64() * self.sample_rate as f64 * self.channels as f64) as usize;
        self.samples_decoded = self.samples_read;
        self.is_eof = false;

        Ok(())
    }

    fn duration(&self) -> Duration {
        self.total_duration
    }

    fn position(&self) -> Duration {
        // Calculate position based on samples read
        let frames = self.samples_read / self.channels as usize;
        Duration::from_secs_f64(frames as f64 / self.sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        self.is_eof && self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_source_implements_audio_source() {
        // This test ensures the trait is implemented
        // Actual functionality requires real audio files
        fn assert_audio_source<T: AudioSource>() {}
        assert_audio_source::<LocalAudioSource>();
    }
}
