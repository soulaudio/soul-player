/// Audio decoder implementation using Symphonia
use crate::error::{AudioError, Result};
use soul_core::{AudioBuffer, AudioDecoder as AudioDecoderTrait, AudioFormat, SampleRate};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Audio decoder using Symphonia
///
/// Supports: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct SymphoniaDecoder;

impl SymphoniaDecoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Self
    }

    /// Convert Symphonia audio buffer to our AudioBuffer format
    fn convert_buffer(decoded: AudioBufferRef, sample_rate: u32) -> Result<AudioBuffer> {
        // Get channel count
        let channels = decoded.spec().channels.count() as u16;

        // Convert to f32 samples (interleaved)
        let samples: Vec<f32> = match decoded {
            AudioBufferRef::F32(buf) => {
                // Already f32, just interleave channels
                let left = buf.chan(0);
                let right = if channels > 1 { buf.chan(1) } else { buf.chan(0) };
                Self::interleave_f32(left, right)
            }
            AudioBufferRef::F64(buf) => {
                // Convert f64 to f32 and interleave
                let left: Vec<f32> = buf.chan(0).iter().map(|&s| s as f32).collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1).iter().map(|&s| s as f32).collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::S32(buf) => {
                // Convert i32 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| s as f32 / i32::MAX as f32)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| s as f32 / i32::MAX as f32)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::S16(buf) => {
                // Convert i16 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| s as f32 / i16::MAX as f32)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::S8(buf) => {
                // Convert i8 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| s as f32 / i8::MAX as f32)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| s as f32 / i8::MAX as f32)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::U32(buf) => {
                // Convert u32 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::U16(buf) => {
                // Convert u16 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::U8(buf) => {
                // Convert u8 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0)
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0)
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::U24(buf) => {
                // Convert u24 to f32 and interleave (unsigned, so scale to [-1, 1])
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| {
                        let val = s.inner() as f32;
                        (val / 8388607.0) * 2.0 - 1.0 // 2^23 - 1, scale to [-1, 1]
                    })
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| {
                            let val = s.inner() as f32;
                            (val / 8388607.0) * 2.0 - 1.0
                        })
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
            AudioBufferRef::S24(buf) => {
                // Convert i24 to f32 and interleave
                let left: Vec<f32> = buf
                    .chan(0)
                    .iter()
                    .map(|&s| {
                        let val = s.inner();
                        val as f32 / 8388607.0 // 2^23 - 1
                    })
                    .collect();
                let right: Vec<f32> = if channels > 1 {
                    buf.chan(1)
                        .iter()
                        .map(|&s| {
                            let val = s.inner();
                            val as f32 / 8388607.0
                        })
                        .collect()
                } else {
                    left.clone()
                };
                Self::interleave_f32(&left, &right)
            }
        };

        let format = AudioFormat::new(SampleRate::new(sample_rate), channels, 32); // 32-bit float

        Ok(AudioBuffer::new(samples, format))
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

impl AudioDecoderTrait for SymphoniaDecoder {
    fn decode(&mut self, path: &Path) -> soul_core::Result<AudioBuffer> {
        // Check if file exists
        if !path.exists() {
            return Err(AudioError::FileNotFound(path.display().to_string()).into());
        }

        // Open the file
        let file =
            std::fs::File::open(path).map_err(|e| soul_core::SoulError::audio(e.to_string()))?;

        // Create media source
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
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
            .map_err(|e| soul_core::SoulError::audio(format!("Failed to create decoder: {}", e)))?;

        // Decode all packets and collect into single buffer
        let mut all_samples = Vec::new();
        let mut channels = 2;

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

            // Decode the packet
            let decoded = decoder
                .decode(&packet)
                .map_err(|e| soul_core::SoulError::audio(format!("Decode error: {}", e)))?;

            channels = decoded.spec().channels.count() as u16;

            // Convert and append to buffer
            let buffer = Self::convert_buffer(decoded, sample_rate)?;
            all_samples.extend_from_slice(&buffer.samples);
        }

        let format = AudioFormat::new(SampleRate::new(sample_rate), channels, 32);

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
