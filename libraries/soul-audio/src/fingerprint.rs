//! Audio fingerprinting using Chromaprint
//!
//! Generates acoustic fingerprints compatible with AcoustID for:
//! - Duplicate detection
//! - Cross-format matching (same audio in FLAC vs MP3)
//! - File relocation tracking
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_audio::fingerprint::{Fingerprinter, FingerprintConfig};
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let fingerprinter = Fingerprinter::new(FingerprintConfig::default());
//! let result = fingerprinter.fingerprint_file(Path::new("song.mp3"))?;
//!
//! println!("Fingerprint: {}", result.fingerprint);
//! println!("Duration: {}s", result.duration_seconds);
//! # Ok(())
//! # }
//! ```

use crate::{AudioError, Result, SymphoniaDecoder};
use rusty_chromaprint::{Configuration, Fingerprinter as ChromaprintFingerprinter};
use soul_core::AudioDecoder;
use std::path::Path;

/// Fingerprint computation result
#[derive(Debug, Clone)]
pub struct FingerprintResult {
    /// The computed fingerprint as a vector of u32 values
    pub fingerprint: Vec<u32>,
    /// Duration of the audio in seconds
    pub duration_seconds: f64,
    /// Sample rate used for fingerprinting
    pub sample_rate: u32,
}

impl FingerprintResult {
    /// Encode fingerprint as base64 string for storage
    #[must_use]
    pub fn to_base64(&self) -> String {
        // Encode fingerprint as raw bytes then base64
        let bytes: Vec<u8> = self
            .fingerprint
            .iter()
            .flat_map(|&v| v.to_le_bytes())
            .collect();
        base64_encode(&bytes)
    }

    /// Decode fingerprint from base64 string
    pub fn from_base64(encoded: &str, duration_seconds: f64) -> Result<Self> {
        let bytes = base64_decode(encoded)?;
        if bytes.len() % 4 != 0 {
            return Err(AudioError::Fingerprint(
                "Invalid fingerprint data length".to_string(),
            ));
        }

        let fingerprint: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        Ok(Self {
            fingerprint,
            duration_seconds,
            sample_rate: FINGERPRINT_SAMPLE_RATE,
        })
    }

    /// Compare two fingerprints and return similarity score (0.0 to 1.0)
    ///
    /// Uses Hamming distance on the fingerprint bits.
    /// Score of 1.0 means identical, 0.0 means completely different.
    #[must_use]
    pub fn similarity(&self, other: &FingerprintResult) -> f64 {
        if self.fingerprint.is_empty() || other.fingerprint.is_empty() {
            return 0.0;
        }

        // Compare overlapping portion
        let len = self.fingerprint.len().min(other.fingerprint.len());
        let mut matching_bits = 0u64;
        let mut total_bits = 0u64;

        for i in 0..len {
            let xor = self.fingerprint[i] ^ other.fingerprint[i];
            // Count matching bits (32 - popcount of XOR)
            matching_bits += 32 - xor.count_ones() as u64;
            total_bits += 32;
        }

        if total_bits == 0 {
            return 0.0;
        }

        matching_bits as f64 / total_bits as f64
    }

    /// Check if two fingerprints likely represent the same audio
    ///
    /// Uses a threshold of 0.85 (85% similarity) which is typical for
    /// matching the same song across different encodings.
    #[must_use]
    pub fn matches(&self, other: &FingerprintResult) -> bool {
        self.similarity(other) >= SIMILARITY_THRESHOLD
    }

    /// Check if fingerprints match with custom threshold
    #[must_use]
    pub fn matches_with_threshold(&self, other: &FingerprintResult, threshold: f64) -> bool {
        self.similarity(other) >= threshold
    }
}

/// Configuration for fingerprint generation
#[derive(Debug, Clone)]
pub struct FingerprintConfig {
    /// Maximum duration to analyze (in seconds). Longer files are truncated.
    /// AcoustID typically uses 120 seconds.
    pub max_duration_seconds: u32,
    /// Whether to use the full duration (ignoring max_duration_seconds)
    pub use_full_duration: bool,
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            max_duration_seconds: 120,
            use_full_duration: false,
        }
    }
}

impl FingerprintConfig {
    /// Create config that analyzes the full audio duration
    #[must_use]
    pub fn full_duration() -> Self {
        Self {
            max_duration_seconds: 0,
            use_full_duration: true,
        }
    }

    /// Create config for quick fingerprinting (first 30 seconds)
    #[must_use]
    pub fn quick() -> Self {
        Self {
            max_duration_seconds: 30,
            use_full_duration: false,
        }
    }
}

/// Audio fingerprinter
pub struct Fingerprinter {
    config: FingerprintConfig,
}

/// Standard sample rate for Chromaprint fingerprinting
const FINGERPRINT_SAMPLE_RATE: u32 = 11025;

/// Number of channels for fingerprinting (mono)
const FINGERPRINT_CHANNELS: u32 = 1;

/// Default similarity threshold for matching
const SIMILARITY_THRESHOLD: f64 = 0.85;

impl Fingerprinter {
    /// Create a new fingerprinter with the given configuration
    #[must_use]
    pub fn new(config: FingerprintConfig) -> Self {
        Self { config }
    }

    /// Generate fingerprint from an audio file
    pub fn fingerprint_file(&self, path: &Path) -> Result<FingerprintResult> {
        // Decode audio
        let mut decoder = SymphoniaDecoder::new();
        let audio = decoder
            .decode(path)
            .map_err(|e| AudioError::Fingerprint(format!("Failed to decode audio: {}", e)))?;

        // Get audio parameters
        let source_rate = audio.format.sample_rate.as_hz();
        let channels = audio.format.channels as u32;

        // Convert to mono if needed and get samples
        let samples = self.prepare_samples(&audio.samples, channels)?;

        // Resample to fingerprint sample rate if needed
        let resampled = if source_rate != FINGERPRINT_SAMPLE_RATE {
            self.resample(&samples, source_rate, FINGERPRINT_SAMPLE_RATE)?
        } else {
            samples
        };

        // Calculate duration
        let duration_seconds = resampled.len() as f64 / FINGERPRINT_SAMPLE_RATE as f64;

        // Truncate if needed
        let max_samples = if self.config.use_full_duration {
            resampled.len()
        } else {
            (self.config.max_duration_seconds as usize) * (FINGERPRINT_SAMPLE_RATE as usize)
        };
        let samples_to_use = &resampled[..resampled.len().min(max_samples)];

        // Generate fingerprint
        let fingerprint = self.compute_fingerprint(samples_to_use)?;

        Ok(FingerprintResult {
            fingerprint,
            duration_seconds,
            sample_rate: FINGERPRINT_SAMPLE_RATE,
        })
    }

    /// Generate fingerprint from raw audio samples
    ///
    /// Samples should be interleaved if multi-channel.
    pub fn fingerprint_samples(
        &self,
        samples: &[f32],
        sample_rate: u32,
        channels: u32,
    ) -> Result<FingerprintResult> {
        // Convert to mono if needed
        let mono = self.prepare_samples(samples, channels)?;

        // Resample to fingerprint sample rate if needed
        let resampled = if sample_rate != FINGERPRINT_SAMPLE_RATE {
            self.resample(&mono, sample_rate, FINGERPRINT_SAMPLE_RATE)?
        } else {
            mono
        };

        // Calculate duration
        let duration_seconds = resampled.len() as f64 / FINGERPRINT_SAMPLE_RATE as f64;

        // Truncate if needed
        let max_samples = if self.config.use_full_duration {
            resampled.len()
        } else {
            (self.config.max_duration_seconds as usize) * (FINGERPRINT_SAMPLE_RATE as usize)
        };
        let samples_to_use = &resampled[..resampled.len().min(max_samples)];

        // Generate fingerprint
        let fingerprint = self.compute_fingerprint(samples_to_use)?;

        Ok(FingerprintResult {
            fingerprint,
            duration_seconds,
            sample_rate: FINGERPRINT_SAMPLE_RATE,
        })
    }

    /// Convert interleaved multi-channel audio to mono
    fn prepare_samples(&self, samples: &[f32], channels: u32) -> Result<Vec<f32>> {
        if channels == 0 {
            return Err(AudioError::Fingerprint("Invalid channel count".to_string()));
        }

        if channels == 1 {
            return Ok(samples.to_vec());
        }

        // Mix down to mono by averaging channels
        let frame_count = samples.len() / channels as usize;
        let mut mono = Vec::with_capacity(frame_count);

        for frame in 0..frame_count {
            let mut sum = 0.0f32;
            for ch in 0..channels as usize {
                sum += samples[frame * channels as usize + ch];
            }
            mono.push(sum / channels as f32);
        }

        Ok(mono)
    }

    /// Simple linear resampling for fingerprinting
    ///
    /// Note: This is a basic resampler suitable for fingerprinting.
    /// For playback, use the high-quality resamplers in the resampling module.
    fn resample(&self, samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
        if from_rate == to_rate {
            return Ok(samples.to_vec());
        }

        let ratio = to_rate as f64 / from_rate as f64;
        let output_len = (samples.len() as f64 * ratio).ceil() as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_pos = i as f64 / ratio;
            let src_idx = src_pos.floor() as usize;
            let frac = src_pos - src_idx as f64;

            let sample = if src_idx + 1 < samples.len() {
                // Linear interpolation
                samples[src_idx] * (1.0 - frac as f32) + samples[src_idx + 1] * frac as f32
            } else if src_idx < samples.len() {
                samples[src_idx]
            } else {
                0.0
            };

            output.push(sample);
        }

        Ok(output)
    }

    /// Compute fingerprint using Chromaprint
    fn compute_fingerprint(&self, samples: &[f32]) -> Result<Vec<u32>> {
        // Convert f32 samples to i16 for Chromaprint
        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * 32767.0) as i16
            })
            .collect();

        // Create Chromaprint fingerprinter with default configuration
        let config = Configuration::preset_test2();
        let mut printer = ChromaprintFingerprinter::new(&config);

        // Start fingerprinting
        printer
            .start(FINGERPRINT_SAMPLE_RATE, FINGERPRINT_CHANNELS)
            .map_err(|e| AudioError::Fingerprint(format!("Failed to start fingerprinter: {e}")))?;

        // Feed samples
        printer.consume(&samples_i16);

        // Finish and get fingerprint
        printer.finish();

        let fingerprint = printer.fingerprint();
        if fingerprint.is_empty() {
            return Err(AudioError::Fingerprint(
                "Failed to generate fingerprint".to_string(),
            ));
        }

        Ok(fingerprint.to_vec())
    }
}

impl Default for Fingerprinter {
    fn default() -> Self {
        Self::new(FingerprintConfig::default())
    }
}

// Simple base64 encoding/decoding without external dependency
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    const DECODE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let data = data.trim_end_matches('=');
    let mut result = Vec::with_capacity(data.len() * 3 / 4);

    let chars: Vec<u8> = data.bytes().collect();
    for chunk in chars.chunks(4) {
        let mut buf = [0u8; 4];
        for (i, &c) in chunk.iter().enumerate() {
            if c >= 128 {
                return Err(AudioError::Fingerprint(
                    "Invalid base64 character".to_string(),
                ));
            }
            let val = DECODE[c as usize];
            if val < 0 {
                return Err(AudioError::Fingerprint(
                    "Invalid base64 character".to_string(),
                ));
            }
            buf[i] = val as u8;
        }

        result.push((buf[0] << 2) | (buf[1] >> 4));
        if chunk.len() > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if chunk.len() > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encoded = base64_encode(&data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_similarity_identical() {
        let fp1 = FingerprintResult {
            fingerprint: vec![0x12345678, 0x9ABCDEF0, 0x11111111],
            duration_seconds: 10.0,
            sample_rate: 11025,
        };
        let fp2 = fp1.clone();
        assert!((fp1.similarity(&fp2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_different() {
        let fp1 = FingerprintResult {
            fingerprint: vec![0x00000000, 0x00000000],
            duration_seconds: 10.0,
            sample_rate: 11025,
        };
        let fp2 = FingerprintResult {
            fingerprint: vec![0xFFFFFFFF, 0xFFFFFFFF],
            duration_seconds: 10.0,
            sample_rate: 11025,
        };
        assert!((fp1.similarity(&fp2) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_partial() {
        let fp1 = FingerprintResult {
            fingerprint: vec![0xFFFF0000, 0xFFFF0000],
            duration_seconds: 10.0,
            sample_rate: 11025,
        };
        let fp2 = FingerprintResult {
            fingerprint: vec![0xFFFF0000, 0x0000FFFF],
            duration_seconds: 10.0,
            sample_rate: 11025,
        };
        // First u32 matches (32 bits), second u32: XOR is all 1s (0 bits match)
        // Total: 32/64 = 0.5
        let sim = fp1.similarity(&fp2);
        assert!((sim - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_fingerprint_config_default() {
        let config = FingerprintConfig::default();
        assert_eq!(config.max_duration_seconds, 120);
        assert!(!config.use_full_duration);
    }

    #[test]
    fn test_mono_conversion() {
        let fingerprinter = Fingerprinter::default();

        // Stereo samples: L=1.0, R=0.5, L=0.5, R=0.5
        let stereo = vec![1.0f32, 0.5, 0.5, 0.5];
        let mono = fingerprinter.prepare_samples(&stereo, 2).unwrap();

        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.75).abs() < 0.001); // (1.0 + 0.5) / 2
        assert!((mono[1] - 0.5).abs() < 0.001); // (0.5 + 0.5) / 2
    }
}
