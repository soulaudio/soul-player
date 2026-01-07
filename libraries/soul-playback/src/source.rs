//! Platform-agnostic audio source trait
//!
//! Abstracts audio decoding for different platforms (desktop, ESP32, etc.)

use crate::error::Result;
use std::time::Duration;

/// Platform-agnostic audio source
///
/// Implementors provide decoded audio samples and seeking functionality.
/// This trait allows PlaybackManager to work with different audio backends
/// (Symphonia on desktop, awedio on ESP32, etc.)
pub trait AudioSource: Send {
    /// Read next chunk of audio samples
    ///
    /// Returns number of samples read (can be less than buffer length at end of track).
    /// Samples are interleaved stereo f32 in [-1.0, 1.0] range.
    ///
    /// # Arguments
    /// * `buffer` - Output buffer for samples (length must be even for stereo)
    ///
    /// # Returns
    /// * `Ok(n)` - Number of samples read (0 = end of track)
    /// * `Err(_)` - Decoding error
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize>;

    /// Seek to position in track
    ///
    /// # Arguments
    /// * `position` - Target position from start of track
    ///
    /// # Returns
    /// * `Ok(())` - Seek successful
    /// * `Err(_)` - Seek failed (position out of range, format doesn't support seek, etc.)
    fn seek(&mut self, position: Duration) -> Result<()>;

    /// Get total track duration
    fn duration(&self) -> Duration;

    /// Get current playback position
    fn position(&self) -> Duration;

    /// Check if track has ended
    ///
    /// Returns true when no more samples are available
    fn is_finished(&self) -> bool;

    /// Reset to beginning of track
    ///
    /// Equivalent to `seek(Duration::ZERO)`
    fn reset(&mut self) -> Result<()> {
        self.seek(Duration::ZERO)
    }
}

/// Dummy audio source for testing
///
/// Generates silence for a specified duration
#[cfg(test)]
pub struct DummyAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
}

#[cfg(test)]
impl DummyAudioSource {
    /// Create new dummy source
    pub fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
        }
    }
}

#[cfg(test)]
impl AudioSource for DummyAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let samples_per_second = self.sample_rate as u64 * 2; // Stereo
        let total_samples = (self.duration.as_secs_f64() * samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * samples_per_second as f64) as u64;

        let remaining = (total_samples - current_sample) as usize;
        let to_read = remaining.min(buffer.len());

        // Fill with silence
        for sample in buffer.iter_mut().take(to_read) {
            *sample = 0.0;
        }

        // Update position
        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.duration {
            return Err(crate::error::PlaybackError::InvalidSeekPosition(position));
        }
        self.position = position;
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        self.position
    }

    fn is_finished(&self) -> bool {
        self.position >= self.duration
    }
}
