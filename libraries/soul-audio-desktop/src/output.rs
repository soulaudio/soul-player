/// CPAL-based audio output implementation (refactored with audio thread)
use crate::error::{AudioOutputError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use soul_core::{AudioBuffer, AudioOutput};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

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
    /// Handle to the audio thread (optional, for joining on drop)
    _audio_thread: Option<JoinHandle<()>>,
}

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

        let sample_rate = config.sample_rate().0;
        let config = config.config();

        Self::with_device_and_config(device, config, sample_rate)
    }

    /// Create a new CPAL output with a specific device and configuration
    fn with_device_and_config(
        device: Device,
        config: StreamConfig,
        sample_rate: u32,
    ) -> Result<Self> {
        let state = Arc::new(AudioState::new());
        let (command_tx, command_rx) = bounded::<AudioCommand>(32);

        // Spawn audio thread
        let state_clone = Arc::clone(&state);
        let audio_thread = thread::spawn(move || {
            Self::audio_thread_run(device, config, state_clone, command_rx);
        });

        Ok(Self {
            command_tx,
            sample_rate,
            state,
            _audio_thread: Some(audio_thread),
        })
    }

    /// Audio thread main loop
    ///
    /// This function runs in a dedicated thread and owns the CPAL Stream.
    /// It processes commands from the main thread via the channel.
    fn audio_thread_run(
        device: Device,
        config: StreamConfig,
        state: Arc<AudioState>,
        command_rx: Receiver<AudioCommand>,
    ) {
        let mut stream: Option<Stream> = None;

        // Process commands
        while let Ok(cmd) = command_rx.recv() {
            match cmd {
                AudioCommand::Play { samples } => {
                    // Stop existing stream
                    if let Some(s) = stream.take() {
                        drop(s);
                    }

                    // Update buffer
                    {
                        let mut buffer_guard = state.buffer.lock().unwrap();
                        *buffer_guard = samples;
                    }

                    // Reset position
                    state.position.store(0, Ordering::Relaxed);

                    // Update state
                    {
                        let mut state_guard = state.state.lock().unwrap();
                        *state_guard = PlaybackState::Playing;
                    }

                    // Create new stream
                    let state_for_callback = Arc::clone(&state);
                    match device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            Self::audio_callback(data, &state_for_callback);
                        },
                        |err| eprintln!("Audio stream error: {}", err),
                        None,
                    ) {
                        Ok(s) => {
                            if s.play().is_ok() {
                                stream = Some(s);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to build stream: {}", e);
                        }
                    }
                }
                AudioCommand::Pause => {
                    if let Some(s) = &stream {
                        let _ = s.pause();
                        let mut state_guard = state.state.lock().unwrap();
                        *state_guard = PlaybackState::Paused;
                    }
                }
                AudioCommand::Resume => {
                    if let Some(s) = &stream {
                        let _ = s.play();
                        let mut state_guard = state.state.lock().unwrap();
                        *state_guard = PlaybackState::Playing;
                    }
                }
                AudioCommand::Stop => {
                    if let Some(s) = stream.take() {
                        drop(s);
                    }
                    let mut state_guard = state.state.lock().unwrap();
                    *state_guard = PlaybackState::Stopped;
                    state.position.store(0, Ordering::Relaxed);
                }
                AudioCommand::SetVolume(vol) => {
                    let mut volume_guard = state.volume.lock().unwrap();
                    *volume_guard = vol;
                }
                AudioCommand::Shutdown => {
                    if let Some(s) = stream.take() {
                        drop(s);
                    }
                    break;
                }
            }
        }
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
            state
                .position
                .store(pos % buffer_len.max(1), Ordering::Relaxed);
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
        // Convert buffer if sample rate doesn't match
        let samples = if buffer.format.sample_rate.as_hz() != self.sample_rate {
            self.resample_buffer(buffer, self.sample_rate)?
        } else {
            buffer.samples.clone()
        };

        // Send play command to audio thread
        self.command_tx
            .send(AudioCommand::Play {
                samples: Arc::new(samples),
            })
            .map_err(|e| {
                soul_core::SoulError::audio(format!("Failed to send play command: {}", e))
            })?;

        Ok(())
    }

    fn pause(&mut self) -> soul_core::Result<()> {
        self.command_tx.send(AudioCommand::Pause).map_err(|e| {
            soul_core::SoulError::audio(format!("Failed to send pause command: {}", e))
        })?;
        Ok(())
    }

    fn resume(&mut self) -> soul_core::Result<()> {
        self.command_tx.send(AudioCommand::Resume).map_err(|e| {
            soul_core::SoulError::audio(format!("Failed to send resume command: {}", e))
        })?;
        Ok(())
    }

    fn stop(&mut self) -> soul_core::Result<()> {
        self.command_tx.send(AudioCommand::Stop).map_err(|e| {
            soul_core::SoulError::audio(format!("Failed to send stop command: {}", e))
        })?;
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> soul_core::Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(AudioOutputError::InvalidVolume(volume).into());
        }

        self.command_tx
            .send(AudioCommand::SetVolume(volume))
            .map_err(|e| {
                soul_core::SoulError::audio(format!("Failed to send volume command: {}", e))
            })?;

        // Also update local state for volume() getter
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
        // Send shutdown command
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        // Audio thread will exit and join handle will be dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul_core::AudioFormat;
    use soul_core::SampleRate;

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
            Err(AudioOutputError::StreamBuildError(_)) => {
                // Also expected in environments without working audio devices
                // (e.g., "device unplugged" in WSL)
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
