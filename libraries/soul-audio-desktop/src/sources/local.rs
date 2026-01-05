//! Local file audio source using Symphonia decoder

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use soul_playback::{AudioSource, PlaybackError, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Audio source for local files
///
/// Uses Symphonia to decode audio files from disk.
/// Supports all formats: MP3, FLAC, OGG, WAV, AAC, OPUS
pub struct LocalAudioSource {
    path: PathBuf,
    sample_rate: u32,
    channels: u16,
    current_position: usize, // Current position in samples buffer
    samples: Vec<f32>,       // Decoded audio samples (interleaved)
    total_duration: Duration,
}

impl LocalAudioSource {
    /// Create a new local audio source
    ///
    /// Decodes the entire audio file into memory for playback.
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    ///
    /// # Returns
    /// * `Ok(source)` - Audio source ready for playback
    /// * `Err(_)` - Failed to open or decode file
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut decoder = SymphoniaDecoder::new();

        // Decode entire file into memory
        let buffer = decoder
            .decode(&path)
            .map_err(|e| PlaybackError::AudioSource(format!("Failed to decode: {}", e)))?;

        let sample_rate = buffer.format.sample_rate.as_hz();
        let channels = buffer.format.channels;
        let samples = buffer.samples; // Already interleaved f32 samples

        // Calculate duration from total samples
        let total_frames = samples.len() / channels as usize;
        let total_duration = Duration::from_secs_f64(total_frames as f64 / sample_rate as f64);

        Ok(Self {
            path,
            sample_rate,
            channels,
            current_position: 0,
            samples,
            total_duration,
        })
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
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        // Check if we've reached the end
        if self.current_position >= self.samples.len() {
            return Ok(0);
        }

        // Calculate how many samples we can read
        let remaining = self.samples.len() - self.current_position;
        let to_read = remaining.min(buffer.len());

        // Copy samples to output buffer
        buffer[..to_read].copy_from_slice(&self.samples[self.current_position..self.current_position + to_read]);

        // Fill remaining buffer with zeros if we don't have enough samples
        if to_read < buffer.len() {
            buffer[to_read..].fill(0.0);
        }

        // Update position
        self.current_position += to_read;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.total_duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }

        // Calculate sample position from time
        let frame_position = (position.as_secs_f64() * self.sample_rate as f64) as usize;
        let sample_position = frame_position * self.channels as usize;

        // Clamp to valid range
        self.current_position = sample_position.min(self.samples.len());

        Ok(())
    }

    fn duration(&self) -> Duration {
        self.total_duration
    }

    fn position(&self) -> Duration {
        // Convert sample position to time
        let frame_position = self.current_position / self.channels as usize;
        Duration::from_secs_f64(frame_position as f64 / self.sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        self.current_position >= self.samples.len()
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
