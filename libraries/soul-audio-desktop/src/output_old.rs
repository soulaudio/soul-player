/// CPAL-based audio output implementation
use crate::error::{AudioOutputError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use crossbeam_channel::{bounded, Sender};
use soul_core::{AudioBuffer, AudioOutput};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Commands sent to the audio thread
enum AudioCommand {
    /// Play a new buffer
    Play { samples: Arc<Vec<f32>> },
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Stop playback
    Stop,
    /// Set volume (0.0 - 1.0)
    SetVolume(f32),
    /// Shutdown the audio thread
    Shutdown,
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackState {
    /// Not playing
    Stopped,
    /// Playing audio
    Playing,
    /// Paused (buffer retained)
    Paused,
}

/// Shared audio state between main thread and audio callback
struct AudioState {
    /// Audio samples (interleaved f32) - Arc for lock-free reading
    buffer: Mutex<Arc<Vec<f32>>>,
    /// Current playback position (in samples, not frames)
    position: AtomicUsize,
    /// Playback state
    state: Mutex<PlaybackState>,
    /// Volume level (0.0 to 1.0)
    volume: Mutex<f32>,
    /// Loop flag
    looping: AtomicBool,
}

impl AudioState {
    fn new() -> Self {
        Self {
            buffer: Mutex::new(Arc::new(Vec::new())),
            position: AtomicUsize::new(0),
            state: Mutex::new(PlaybackState::Stopped),
            volume: Mutex::new(1.0),
            looping: AtomicBool::new(false),
        }
    }
}

/// CPAL audio output
///
/// Implements the `AudioOutput` trait using CPAL for cross-platform audio output.
///
/// **Architecture**: Uses a dedicated audio thread that owns the CPAL Stream.
/// The main thread communicates with the audio thread via channels, avoiding
/// Send/Sync issues with CPAL's Stream type across different platforms.
pub struct CpalOutput {
    /// Channel to send commands to the audio thread
    command_tx: Sender<AudioCommand>,
    /// Sample rate of the output device
    sample_rate: u32,
    /// Shared state for volume tracking
    state: Arc<AudioState>,
}

// CpalOutput is Send because:
// - Sender<AudioCommand> is Send
// - u32 is Send
// - Arc<AudioState> is Send
// The audio thread owns the Stream, so we don't need to worry about Stream's Send impl

impl CpalOutput {
    /// Create a new CPAL output using the default audio device
    ///
    /// # Errors
    /// Returns an error if no audio device is found or configuration fails
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioOutputError::DeviceNotFound)?;

        // Get the default output configuration
        let config = device
            .default_output_config()
            .map_err(|e| AudioOutputError::StreamBuildError(e.to_string()))?;

        let config = config.config();

        Ok(Self {
            device,
            config,
            stream: Arc::new(Mutex::new(None)),
            state: Arc::new(AudioState::new()),
        })
    }

    /// Create a new CPAL output with a specific device and configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub fn with_config(device: Device, config: StreamConfig) -> Result<Self> {
        Ok(Self {
            device,
            config,
            stream: Arc::new(Mutex::new(None)),
            state: Arc::new(AudioState::new()),
        })
    }

    /// Build the output stream
    fn build_stream(&mut self, buffer: &AudioBuffer) -> Result<()> {
        let _channels = self.config.channels as usize;
        let sample_rate = self.config.sample_rate.0;

        // Convert buffer if sample rate doesn't match
        let converted_buffer = if buffer.format.sample_rate.as_hz() != sample_rate {
            self.resample_buffer(buffer, sample_rate)?
        } else {
            buffer.samples.clone()
        };

        // Store the buffer in shared state
        {
            let mut buffer_guard = self.state.buffer.lock().unwrap();
            *buffer_guard = Arc::new(converted_buffer);
        }

        // Reset position
        self.state.position.store(0, Ordering::Relaxed);

        // Update playback state
        {
            let mut state_guard = self.state.state.lock().unwrap();
            *state_guard = PlaybackState::Playing;
        }

        // Create the stream
        let state_for_callback = Arc::clone(&self.state);
        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    Self::audio_callback(data, &state_for_callback);
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| AudioOutputError::StreamBuildError(e.to_string()))?;

        // Play the stream
        stream
            .play()
            .map_err(|e| AudioOutputError::PlayError(e.to_string()))?;

        // Store the stream in the mutex
        let mut stream_guard = self.stream.lock().unwrap();
        *stream_guard = Some(stream);

        Ok(())
    }

    /// Audio callback function (runs in real-time audio thread)
    fn audio_callback(output: &mut [f32], state: &AudioState) {
        // Check playback state
        let playback_state = *state.state.lock().unwrap();
        if playback_state != PlaybackState::Playing {
            // Fill with silence
            output.fill(0.0);
            return;
        }

        // Get volume
        let volume = *state.volume.lock().unwrap();

        // Get buffer reference (Arc clone is cheap)
        let buffer = {
            let buffer_guard = state.buffer.lock().unwrap();
            Arc::clone(&*buffer_guard)
        };

        // Get current position
        let mut pos = state.position.load(Ordering::Relaxed);
        let buffer_len = buffer.len();

        if buffer_len == 0 {
            output.fill(0.0);
            return;
        }

        // Fill output buffer
        for out_sample in output.iter_mut() {
            if pos >= buffer_len {
                if state.looping.load(Ordering::Relaxed) {
                    pos = 0;
                } else {
                    *out_sample = 0.0;
                    continue;
                }
            }

            *out_sample = buffer[pos] * volume;
            pos += 1;
        }

        // Update position
        if pos >= buffer_len && !state.looping.load(Ordering::Relaxed) {
            // Reached end, stop playback
            let mut playback_state = state.state.lock().unwrap();
            *playback_state = PlaybackState::Stopped;
        } else {
            state.position.store(pos % buffer_len.max(1), Ordering::Relaxed);
        }
    }

    /// Resample audio buffer to target sample rate
    fn resample_buffer(&self, buffer: &AudioBuffer, target_rate: u32) -> Result<Vec<f32>> {
        use rubato::{
            Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
            WindowFunction,
        };

        let source_rate = buffer.format.sample_rate.as_hz();
        let channels = buffer.format.channels as usize;

        // Create resampler
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler = SincFixedIn::<f32>::new(
            target_rate as f64 / source_rate as f64,
            2.0,
            params,
            buffer.frames(),
            channels,
        )
        .map_err(|e| AudioOutputError::ResampleError(e.to_string()))?;

        // Deinterleave input samples
        let mut deinterleaved = vec![Vec::with_capacity(buffer.frames()); channels];
        for frame_idx in 0..buffer.frames() {
            for ch in 0..channels {
                deinterleaved[ch].push(buffer.samples[frame_idx * channels + ch]);
            }
        }

        // Resample
        let resampled = resampler
            .process(&deinterleaved, None)
            .map_err(|e| AudioOutputError::ResampleError(e.to_string()))?;

        // Interleave output samples
        let output_frames = resampled[0].len();
        let mut interleaved = Vec::with_capacity(output_frames * channels);
        for frame_idx in 0..output_frames {
            for ch in 0..channels {
                interleaved.push(resampled[ch][frame_idx]);
            }
        }

        Ok(interleaved)
    }
}

impl AudioOutput for CpalOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> soul_core::Result<()> {
        // Stop any existing stream
        {
            let stream_guard = self.stream.lock().unwrap();
            if stream_guard.is_some() {
                drop(stream_guard);
                self.stop()?;
            }
        }

        // Convert buffer if needed and build stream
        self.build_stream(buffer)?;

        Ok(())
    }

    fn pause(&mut self) -> soul_core::Result<()> {
        let stream_guard = self.stream.lock().unwrap();
        if let Some(stream) = stream_guard.as_ref() {
            stream
                .pause()
                .map_err(|e| soul_core::SoulError::audio(format!("Failed to pause: {}", e)))?;

            let mut state = self.state.state.lock().unwrap();
            *state = PlaybackState::Paused;
        }

        Ok(())
    }

    fn resume(&mut self) -> soul_core::Result<()> {
        let stream_guard = self.stream.lock().unwrap();
        if let Some(stream) = stream_guard.as_ref() {
            stream
                .play()
                .map_err(|e| soul_core::SoulError::audio(format!("Failed to resume: {}", e)))?;

            let mut state = self.state.state.lock().unwrap();
            *state = PlaybackState::Playing;
        }

        Ok(())
    }

    fn stop(&mut self) -> soul_core::Result<()> {
        let mut stream_guard = self.stream.lock().unwrap();
        if let Some(stream) = stream_guard.take() {
            drop(stream);
        }

        let mut state = self.state.state.lock().unwrap();
        *state = PlaybackState::Stopped;
        self.state.position.store(0, Ordering::Relaxed);

        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> soul_core::Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(AudioOutputError::InvalidVolume(volume).into());
        }

        let mut vol = self.state.volume.lock().unwrap();
        *vol = volume;

        Ok(())
    }

    fn volume(&self) -> f32 {
        *self.state.volume.lock().unwrap()
    }
}

impl Drop for CpalOutput {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul_core::AudioFormat;

    #[test]
    fn create_output() {
        // This test might fail in CI without audio devices
        match CpalOutput::new() {
            Ok(output) => {
                assert_eq!(output.volume(), 1.0);
            }
            Err(AudioOutputError::DeviceNotFound) => {
                // Expected in headless environments
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn volume_control() {
        let mut output = match CpalOutput::new() {
            Ok(o) => o,
            Err(_) => return, // Skip test if no device
        };

        assert!(output.set_volume(0.5).is_ok());
        assert_eq!(output.volume(), 0.5);

        assert!(output.set_volume(0.0).is_ok());
        assert_eq!(output.volume(), 0.0);

        assert!(output.set_volume(1.0).is_ok());
        assert_eq!(output.volume(), 1.0);

        // Invalid volumes
        assert!(output.set_volume(-0.1).is_err());
        assert!(output.set_volume(1.1).is_err());
    }

    #[test]
    fn playback_silence() {
        let mut output = match CpalOutput::new() {
            Ok(o) => o,
            Err(_) => return, // Skip test if no device
        };

        // Create a silent buffer
        let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
        let buffer = AudioBuffer::new(vec![0.0; 44100 * 2], format); // 1 second of silence

        // This should not error
        assert!(output.play(&buffer).is_ok());

        // Give it a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(output.pause().is_ok());
        assert!(output.resume().is_ok());
        assert!(output.stop().is_ok());
    }
}
